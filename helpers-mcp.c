/*
 * helpers-mcp — tiny C client shim for the Helpers MCP server.
 *
 * Why C: an MCP-capable agent spawns this once per session and waits for it.
 * A C binary starts in ~1ms (no V8), so the only cost left is connecting to an
 * already-warm Node daemon (helpers-serverd.js). After the first launch
 * there is effectively no startup time, and the daemon "always runs when
 * necessary" — this shim spawns it on demand and it idle-exits when unused.
 *
 * Behavior:
 *   1. Derive a per-workspace socket path from cwd plus the HELPERS_* env,
 *      so different projects/configs get isolated, correctly-scoped daemons.
 *   2. Connect. If no daemon is listening, spawn one (detached, inheriting our
 *      cwd+env so its scope matches) and wait briefly for it.
 *   3. Transparently proxy stdin<->socket: standard stdio MCP, so every agent
 *      provider works unchanged.
 *   4. If the daemon can't be reached at all, exec the Node stdio server
 *      directly — the shim can never be worse than running node directly.
 *
 * Compile-time paths (override with -D at build, or env at runtime):
 *   NODE_BIN    node executable           (env HELPERS_NODE_BIN)
 *   DAEMON_JS   helpers-serverd.js (env HELPERS_DAEMON_JS)
 *   STDIO_JS    helpers-server     (env HELPERS_STDIO_JS)
 */

#include <stdio.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <time.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/stat.h>
#include <sys/types.h>

#ifndef NODE_BIN
#define NODE_BIN "node"
#endif
#ifndef DAEMON_JS
#define DAEMON_JS ""
#endif
#ifndef STDIO_JS
#define STDIO_JS ""
#endif

extern char **environ;

static int g_debug = 0;
static void dbg(const char *fmt, ...) {
    if (!g_debug) return;
    va_list ap;
    va_start(ap, fmt);
    fputs("[helpers-mcp] ", stderr);
    vfprintf(stderr, fmt, ap);
    fputc('\n', stderr);
    va_end(ap);
}

static const char *envdef(const char *name, const char *fallback) {
    const char *v = getenv(name);
    return (v && *v) ? v : fallback;
}

/* FNV-1a 64-bit, rendered hex. Stable across runs for identical context. */
static unsigned long long fnv1a(unsigned long long h, const char *s) {
    for (; *s; s++) {
        h ^= (unsigned char)*s;
        h *= 1099511628211ULL;
    }
    return h;
}

/* Socket path = ~/.cache/helpers/mcpd-<hash(cwd + relevant env)>.sock */
static void compute_socket(char *out, size_t n, const char *home) {
    char cwd[4096];
    if (!getcwd(cwd, sizeof cwd)) strcpy(cwd, "?");
    unsigned long long h = 1469598103934665603ULL;
    h = fnv1a(h, cwd);
    for (char **e = environ; *e; e++) {
        /* Fold in every HELPERS_* var (prefix is 8 chars incl. the underscore). */
        if (strncmp(*e, "HELPERS_", 8) == 0) {
            /* HELPERS_MCPD_SOCK is derived output, not input — don't fold it in. */
            if (strncmp(*e, "HELPERS_MCPD_SOCK=", 18) == 0) continue;
            h = fnv1a(h, *e);
        }
    }
    snprintf(out, n, "%s/.cache/helpers/mcpd-%016llx.sock", home, h);
}

static void ensure_cache_dir(const char *home) {
    char p[4096];
    snprintf(p, sizeof p, "%s/.cache", home);
    mkdir(p, 0700);
    snprintf(p, sizeof p, "%s/.cache/helpers", home);
    mkdir(p, 0700);
}

static int connect_socket(const char *path) {
    int fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (fd < 0) return -1;
    struct sockaddr_un addr;
    memset(&addr, 0, sizeof addr);
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, path, sizeof addr.sun_path - 1);
    if (connect(fd, (struct sockaddr *)&addr, sizeof addr) == 0) return fd;
    close(fd);
    return -1;
}

static void spawn_daemon(const char *node, const char *daemon_js,
                         const char *sock, const char *home) {
    pid_t pid = fork();
    if (pid != 0) return; /* parent (or fork failed): just continue */

    /* child: fully detach */
    setsid();
    int devnull = open("/dev/null", O_RDWR);
    if (devnull >= 0) { dup2(devnull, 0); dup2(devnull, 1); }
    char logp[4096];
    snprintf(logp, sizeof logp, "%s/.cache/helpers/mcpd.log", home);
    int log = open(logp, O_WRONLY | O_CREAT | O_APPEND, 0600);
    dup2(log >= 0 ? log : devnull, 2);
    if (devnull > 2) close(devnull);
    if (log > 2) close(log);

    setenv("HELPERS_MCPD_SOCK", sock, 1);
    char *argv[] = {(char *)node, (char *)daemon_js, NULL};
    execv(node, argv);                 /* baked/abs path */
    char *argv2[] = {"node", (char *)daemon_js, NULL};
    execvp("node", argv2);             /* PATH fallback */
    _exit(127);
}

static ssize_t write_all(int fd, const char *buf, size_t len) {
    size_t off = 0;
    while (off < len) {
        ssize_t w = write(fd, buf + off, len - off);
        if (w < 0) {
            if (errno == EINTR) continue;
            return -1;
        }
        off += (size_t)w;
    }
    return (ssize_t)off;
}

/* Bidirectional proxy: stdin->sock, sock->stdout. Returns on either EOF. */
static void proxy(int sock) {
    char buf[65536];
    struct pollfd fds[2];
    fds[0].fd = 0;     fds[0].events = POLLIN;  /* stdin  */
    fds[1].fd = sock;  fds[1].events = POLLIN;  /* socket */
    int stdin_open = 1;
    for (;;) {
        if (poll(fds, 2, -1) < 0) {
            if (errno == EINTR) continue;
            return;
        }
        /* Drain the daemon first so no response is lost on shutdown. */
        if (fds[1].revents & POLLIN) {
            ssize_t r = read(sock, buf, sizeof buf);
            if (r <= 0) return;
            if (write_all(1, buf, (size_t)r) < 0) return;
        } else if (fds[1].revents & (POLLHUP | POLLERR)) {
            return;
        }
        if (stdin_open && (fds[0].revents & POLLIN)) {
            ssize_t r = read(0, buf, sizeof buf);
            if (r <= 0) {
                shutdown(sock, SHUT_WR);
                stdin_open = 0;
                fds[0].fd = -1;
            } else if (write_all(sock, buf, (size_t)r) < 0) {
                return;
            }
        } else if (stdin_open && (fds[0].revents & (POLLHUP | POLLERR))) {
            shutdown(sock, SHUT_WR);
            stdin_open = 0;
            fds[0].fd = -1;
        }
    }
}

static void exec_stdio_fallback(const char *node, const char *stdio_js) {
    if (!stdio_js || !*stdio_js) {
        fprintf(stderr, "helpers-mcp: no daemon and no stdio fallback configured\n");
        _exit(1);
    }
    char *argv[] = {(char *)node, (char *)stdio_js, NULL};
    execv(node, argv);
    char *argv2[] = {"node", (char *)stdio_js, NULL};
    execvp("node", argv2);
    fprintf(stderr, "helpers-mcp: failed to exec node: %s\n", strerror(errno));
    _exit(127);
}

int main(void) {
    const char *home = getenv("HOME");
    if (!home || !*home) home = "/tmp";
    const char *node = envdef("HELPERS_NODE_BIN", NODE_BIN);
    const char *daemon_js = envdef("HELPERS_DAEMON_JS", DAEMON_JS);
    const char *stdio_js = envdef("HELPERS_STDIO_JS", STDIO_JS);

    g_debug = getenv("HELPERS_MCP_DEBUG") != NULL;

    char sock[4096];
    compute_socket(sock, sizeof sock, home);
    ensure_cache_dir(home);
    dbg("home=%s node=%s", home, node);
    dbg("daemon_js=%s", daemon_js);
    dbg("socket=%s", sock);

    /* Fast path: a warm daemon is already listening. */
    int fd = connect_socket(sock);
    dbg("initial connect fd=%d", fd);

    if (fd < 0 && daemon_js && *daemon_js) {
        dbg("spawning background server...");
        spawn_daemon(node, daemon_js, sock, home);
        /* Give the background server a short window to bind (it normally takes
         * ~200ms). If it isn't ready quickly we DON'T keep waiting — we just run
         * the server directly so startup is always bounded and reliable. The
         * background server keeps coming up for next time. */
        for (int i = 0; i < 200 && fd < 0; i++) {
            struct timespec ts = {0, 10 * 1000 * 1000}; /* 10ms * 200 = ~2s cap */
            nanosleep(&ts, NULL);
            fd = connect_socket(sock);
        }
        dbg("post-spawn connect fd=%d", fd);
    }

    if (fd >= 0) {
        dbg("proxying to background server");
        proxy(fd);
        close(fd);
        return 0;
    }

    /* Background server didn't answer in time — run directly. Never worse than
     * launching node yourself; always produces a working server. */
    dbg("falling back to direct node");
    exec_stdio_fallback(node, stdio_js);
    return 1;
}
