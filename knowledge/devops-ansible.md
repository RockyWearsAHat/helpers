# Ansible — Agentless Configuration Management

## Architecture

Ansible is agentless — connects to managed nodes via SSH (Linux) or WinRM (Windows), pushes modules, executes them, and removes them. No daemon or agent installed on targets.

```
┌──────────────┐    SSH/WinRM    ┌──────────────┐
│  Control     │───────────────►│  Managed     │
│  Node        │                │  Node(s)     │
│              │  push module   │              │
│  ansible     │  execute       │  Python 2.7+ │
│  ansible-    │  return JSON   │  (or raw for  │
│  playbook    │  clean up      │   no Python)  │
└──────────────┘                └──────────────┘
```

**Key concepts:**

- **Control node** — where Ansible runs (Linux/macOS, not Windows natively)
- **Managed node** — target system being configured
- **Inventory** — list of managed nodes
- **Module** — unit of work (install package, copy file, start service)
- **Task** — single module invocation with parameters
- **Play** — maps a group of hosts to tasks
- **Playbook** — YAML file containing one or more plays
- **Role** — reusable, structured bundle of tasks/files/templates/variables
- **Collection** — distribution format for roles, modules, plugins

## Inventory

### Static Inventory

```ini
# inventory/hosts.ini
[webservers]
web1.example.com ansible_host=10.0.1.10
web2.example.com ansible_host=10.0.1.11

[dbservers]
db1.example.com ansible_host=10.0.2.10 ansible_port=2222

[production:children]
webservers
dbservers

[production:vars]
ansible_user=deploy
ansible_ssh_private_key_file=~/.ssh/prod_key
```

YAML format (equivalent):

```yaml
# inventory/hosts.yml
all:
  children:
    webservers:
      hosts:
        web1.example.com:
          ansible_host: 10.0.1.10
        web2.example.com:
          ansible_host: 10.0.1.11
    dbservers:
      hosts:
        db1.example.com:
          ansible_host: 10.0.2.10
          ansible_port: 2222
    production:
      children:
        webservers:
        dbservers:
      vars:
        ansible_user: deploy
```

### Dynamic Inventory

Scripts or plugins that generate inventory from cloud APIs, CMDB, etc:

```bash
# AWS EC2 dynamic inventory (plugin)
# inventory/aws_ec2.yml
plugin: amazon.aws.aws_ec2
regions:
  - us-east-1
  - us-west-2
keyed_groups:
  - key: tags.Environment
    prefix: env
  - key: instance_type
    prefix: type
filters:
  tag:ManagedBy: ansible
compose:
  ansible_host: private_ip_address
```

```bash
ansible-inventory -i inventory/aws_ec2.yml --list  # verify
```

### Host & Group Variables

```
inventory/
  hosts.yml
  host_vars/
    web1.example.com.yml    # variables specific to web1
    db1.example.com.yml
  group_vars/
    all.yml                 # applies to all hosts
    webservers.yml          # applies to webservers group
    production.yml
```

## Playbooks

```yaml
# site.yml
---
- name: Configure web servers
  hosts: webservers
  become: true # sudo
  gather_facts: true # collect system info (default: true)
  vars:
    http_port: 80
    app_version: "2.1.0"

  pre_tasks:
    - name: Update apt cache
      ansible.builtin.apt:
        update_cache: true
        cache_valid_time: 3600
      when: ansible_os_family == "Debian"

  roles:
    - common
    - role: nginx
      vars:
        nginx_port: "{{ http_port }}"

  tasks:
    - name: Deploy application
      ansible.builtin.copy:
        src: "app-{{ app_version }}.tar.gz"
        dest: /opt/app/
      notify: Restart app

    - name: Ensure app is running
      ansible.builtin.service:
        name: myapp
        state: started
        enabled: true

  handlers:
    - name: Restart app
      ansible.builtin.service:
        name: myapp
        state: restarted

  post_tasks:
    - name: Verify health endpoint
      ansible.builtin.uri:
        url: "http://localhost:{{ http_port }}/health"
        status_code: 200
      retries: 5
      delay: 3
```

**Execution order within a play:** `pre_tasks` → `pre_tasks handlers` → `roles` → `tasks` → `tasks handlers` → `post_tasks` → `post_tasks handlers`.

### Task Control

```yaml
# Conditionals
- name: Install on Debian
  ansible.builtin.apt:
    name: nginx
  when: ansible_os_family == "Debian"

- name: Install on RedHat
  ansible.builtin.yum:
    name: nginx
  when: ansible_os_family == "RedHat"

# Loops
- name: Create users
  ansible.builtin.user:
    name: "{{ item.name }}"
    groups: "{{ item.groups }}"
    state: present
  loop:
    - { name: alice, groups: "admin,docker" }
    - { name: bob, groups: "developers" }

# Loop with dict2items
- name: Set sysctl values
  ansible.posix.sysctl:
    name: "{{ item.key }}"
    value: "{{ item.value }}"
  loop: "{{ sysctl_settings | dict2items }}"

# Register output and use it
- name: Check disk space
  ansible.builtin.command: df -h /
  register: disk_result
  changed_when: false

- name: Warn if low
  ansible.builtin.debug:
    msg: "Low disk space!"
  when: "'90%' in disk_result.stdout"

# Error handling
- name: Risky operation
  ansible.builtin.command: /opt/risky.sh
  ignore_errors: true
  register: risky_result

- block:
    - name: Try something
      ansible.builtin.command: /opt/fragile.sh
  rescue:
    - name: Handle failure
      ansible.builtin.debug:
        msg: "Task failed, running recovery"
  always:
    - name: Always do this
      ansible.builtin.debug:
        msg: "Cleanup complete"

# Tags
- name: Install packages
  ansible.builtin.apt:
    name: "{{ item }}"
  loop: [nginx, curl, htop]
  tags: [packages, setup]
```

```bash
ansible-playbook site.yml --tags packages      # only tagged tasks
ansible-playbook site.yml --skip-tags setup     # skip tagged tasks
ansible-playbook site.yml --limit webservers    # subset of hosts
ansible-playbook site.yml --check               # dry run
ansible-playbook site.yml --diff                # show file changes
ansible-playbook site.yml --step                # confirm each task
```

## Core Modules

| Module                | Purpose                              | Example                                                  |
| --------------------- | ------------------------------------ | -------------------------------------------------------- |
| `command`             | Run command (no shell features)      | `command: ls /tmp`                                       |
| `shell`               | Run command (shell pipes, redirects) | `shell: cat /etc/passwd \| grep root`                    |
| `raw`                 | Run command without Python on target | `raw: apt-get install -y python3`                        |
| `copy`                | Copy file to remote                  | `copy: src=app.conf dest=/etc/`                          |
| `template`            | Render Jinja2 template to remote     | `template: src=nginx.conf.j2 dest=/etc/nginx/nginx.conf` |
| `file`                | Manage file/directory properties     | `file: path=/opt/app state=directory mode=0755`          |
| `lineinfile`          | Ensure line in file                  | `lineinfile: path=/etc/hosts line="10.0.1.5 db"`         |
| `apt` / `yum` / `dnf` | Package management                   | `apt: name=nginx state=present`                          |
| `package`             | OS-agnostic package management       | `package: name=git state=latest`                         |
| `service` / `systemd` | Service management                   | `service: name=nginx state=restarted`                    |
| `user` / `group`      | User/group management                | `user: name=deploy groups=docker`                        |
| `git`                 | Git operations                       | `git: repo=https://... dest=/opt/app version=main`       |
| `uri`                 | HTTP requests                        | `uri: url=http://localhost/health status_code=200`       |
| `debug`               | Print variables/messages             | `debug: var=my_variable`                                 |
| `assert`              | Validate conditions                  | `assert: that: "result.rc == 0"`                         |
| `wait_for`            | Wait for condition                   | `wait_for: port=443 delay=5 timeout=60`                  |

**Idempotency:** Modules report `changed` or `ok`. Most built-in modules are idempotent — running twice produces the same result. `command` and `shell` always report `changed` unless you add `changed_when:` or `creates:`/`removes:`.

## Variables & Facts

### Variable Precedence (Ascending — Higher Wins)

1. Role defaults (`roles/x/defaults/main.yml`)
2. Inventory group_vars/all
3. Inventory group_vars/\*
4. Inventory host_vars/\*
5. Play vars
6. Play vars_files
7. Role vars (`roles/x/vars/main.yml`)
8. Block vars
9. Task vars
10. set_fact / registered vars
11. Extra vars (`-e` / `--extra-vars`) — **always wins**

### Facts

Gathered automatically at play start (`gather_facts: true`). Access as `ansible_*` variables:

```yaml
ansible_hostname          # short hostname
ansible_fqdn              # fully qualified domain name
ansible_os_family         # Debian, RedHat, Suse, etc.
ansible_distribution      # Ubuntu, CentOS, etc.
ansible_distribution_version  # 22.04, 8.5, etc.
ansible_memtotal_mb       # total RAM
ansible_processor_vcpus   # CPU count
ansible_default_ipv4.address  # primary IP
```

### Ansible Vault

Encrypt sensitive variables:

```bash
# Encrypt a file
ansible-vault encrypt group_vars/production/vault.yml

# Encrypt a single string
ansible-vault encrypt_string 'my_secret_password' --name 'db_password'

# Edit encrypted file
ansible-vault edit group_vars/production/vault.yml

# Run playbook with vault password
ansible-playbook site.yml --ask-vault-pass
ansible-playbook site.yml --vault-password-file ~/.vault_pass
```

Convention: prefix encrypted variable names with `vault_`, reference in unencrypted vars:

```yaml
# group_vars/production/vault.yml (encrypted)
vault_db_password: supersecret

# group_vars/production/vars.yml (plain)
db_password: "{{ vault_db_password }}"
```

## Jinja2 Templates

```jinja
{# templates/nginx.conf.j2 #}
server {
    listen {{ http_port | default(80) }};
    server_name {{ ansible_fqdn }};

    {% if ssl_enabled %}
    listen 443 ssl;
    ssl_certificate     {{ ssl_cert_path }};
    ssl_certificate_key {{ ssl_key_path }};
    {% endif %}

    {% for backend in app_backends %}
    upstream {{ backend.name }} {
        {% for server in backend.servers %}
        server {{ server.host }}:{{ server.port }} weight={{ server.weight | default(1) }};
        {% endfor %}
    }
    {% endfor %}

    location / {
        proxy_pass http://{{ app_backends[0].name }};
    }
}
```

**Useful filters:** `default()`, `mandatory`, `int`, `bool`, `join(',')`, `to_json`, `to_yaml`, `regex_replace()`, `selectattr()`, `map()`, `flatten`, `unique`, `combine()`.

## Roles

Structured, reusable units:

```
roles/
  nginx/
    defaults/main.yml     # low-priority default vars
    vars/main.yml         # high-priority vars
    tasks/main.yml        # task list
    handlers/main.yml     # handlers
    templates/            # Jinja2 templates
    files/                # static files
    meta/main.yml         # role dependencies, metadata
    molecule/             # test scenarios
```

### Role Dependencies

```yaml
# roles/app/meta/main.yml
dependencies:
  - role: common
  - role: nginx
    vars:
      nginx_port: 8080
```

## Collections

Namespace packaging for modules, roles, and plugins. Distribution via Ansible Galaxy or private Automation Hub.

```bash
# Install collection
ansible-galaxy collection install community.docker
ansible-galaxy collection install -r requirements.yml

# requirements.yml
collections:
  - name: community.docker
    version: ">=3.0.0"
  - name: amazon.aws
    version: "6.5.0"
```

Use FQCN (Fully Qualified Collection Name) in playbooks:

```yaml
- name: Run container
  community.docker.docker_container:
    name: myapp
    image: myapp:latest
    ports: ["8080:80"]
```

## Testing with Molecule

Test roles in isolated environments (Docker, Podman, Vagrant, cloud):

```bash
# Initialize molecule scenario for a role
cd roles/nginx
molecule init scenario --driver-name docker

# Directory structure created
molecule/
  default/
    molecule.yml       # test config
    converge.yml       # playbook that applies the role
    verify.yml         # assertions
    prepare.yml        # pre-test setup (optional)
```

```yaml
# molecule/default/molecule.yml
driver:
  name: docker
platforms:
  - name: ubuntu-22
    image: ubuntu:22.04
    pre_build_image: true
  - name: rocky-9
    image: rockylinux:9
    pre_build_image: true
provisioner:
  name: ansible
verifier:
  name: ansible # or testinfra
```

```bash
molecule test              # full cycle: create → converge → verify → destroy
molecule converge          # just apply the role
molecule verify            # just run assertions
molecule login -h ubuntu-22  # SSH into test container
molecule destroy           # clean up
```

## AWX / Automation Controller (Tower)

Web UI + REST API + RBAC + scheduling + credential management on top of Ansible:

- **Projects** — Git repos containing playbooks
- **Inventories** — managed host lists (static, dynamic, smart)
- **Templates** — job definitions (playbook + inventory + credentials + extra vars)
- **Workflows** — chain templates with conditional logic
- **Credentials** — encrypted storage for SSH keys, cloud tokens, vault passwords
- **RBAC** — organizations, teams, user-level permissions per resource
- **Schedules** — cron-like recurring job execution
- **Notifications** — Slack, email, webhook on job completion/failure

Execution flow: User launches template → AWX creates job → job runs on execution environment (container with Ansible + collections) → results stored in database → UI/API show output.
