'use strict';

/**
 * neural.js — Ambient neural-network canvas animation for ATLAS
 *
 * Renders a faint particle network fixed behind all UI surfaces.
 * Visible through the hero's torn bottom edge and content gaps.
 * Deliberately low-contrast: intelligence suggested, not screamed.
 */
(function () {
  var canvas = document.getElementById('neural-canvas');
  if (!canvas || !canvas.getContext) return;

  var ctx = canvas.getContext('2d');

  var PARTICLE_COUNT = 52;
  var CONNECTION_DIST = 175;
  var BASE_SPEED = 0.22;

  var W = 0, H = 0;
  var particles = [];
  var animId = null;
  var isVisible = true;
  var parallaxRafPending = false;
  var PARALLAX_FACTOR = 0.32;

  function isDark() {
    return document.documentElement.getAttribute('data-theme') !== 'light';
  }

  function resize() {
    W = canvas.width = window.innerWidth;
    H = canvas.height = window.innerHeight;
  }

  function makeParticle() {
    var angle = Math.random() * Math.PI * 2;
    var speed = BASE_SPEED * (0.5 + Math.random() * 0.8);
    return {
      x: Math.random() * W,
      y: Math.random() * H,
      vx: Math.cos(angle) * speed,
      vy: Math.sin(angle) * speed,
      r: 1.0 + Math.random() * 1.4,
      pulse: Math.random() * Math.PI * 2, /* phase offset for pulsing */
    };
  }

  function init() {
    resize();
    particles = [];
    for (var i = 0; i < PARTICLE_COUNT; i++) {
      particles.push(makeParticle());
    }
  }

  function applyParallax() {
    parallaxRafPending = false;
    var y = window.scrollY || window.pageYOffset || 0;
    canvas.style.transform = 'translate3d(0,' + (-y * PARALLAX_FACTOR) + 'px,0)';
  }

  function requestParallaxUpdate() {
    if (parallaxRafPending) return;
    parallaxRafPending = true;
    requestAnimationFrame(applyParallax);
  }

  var lastTime = 0;

  function tick(now) {
    animId = requestAnimationFrame(tick);

    if (!isVisible) return;

    /* Throttle to ~30fps for energy efficiency */
    if (now - lastTime < 33) return;
    lastTime = now;

    ctx.clearRect(0, 0, W, H);

    var dark = isDark();

    /* Blue tones only — no vaporwave RGB */
    var nodeAlpha = dark ? 0.22 : 0.14;
    var lineAlphaMax = dark ? 0.16 : 0.10;
    var nodeR = dark ? 77 : 45;
    var nodeG = dark ? 127 : 94;
    var nodeB = dark ? 255 : 232;

    var t = now * 0.001;

    /* Move particles */
    for (var i = 0; i < particles.length; i++) {
      var p = particles[i];
      p.x += p.vx;
      p.y += p.vy;

      /* Soft wrap at edges (no hard bounce) */
      if (p.x < -20) p.x = W + 20;
      if (p.x > W + 20) p.x = -20;
      if (p.y < -20) p.y = H + 20;
      if (p.y > H + 20) p.y = -20;
    }

    /* Draw connections */
    for (var i = 0; i < particles.length - 1; i++) {
      for (var j = i + 1; j < particles.length; j++) {
        var dx = particles[i].x - particles[j].x;
        var dy = particles[i].y - particles[j].y;
        var distSq = dx * dx + dy * dy;
        if (distSq < CONNECTION_DIST * CONNECTION_DIST) {
          var dist = Math.sqrt(distSq);
          var alpha = (1 - dist / CONNECTION_DIST) * lineAlphaMax;
          ctx.beginPath();
          ctx.moveTo(particles[i].x, particles[i].y);
          ctx.lineTo(particles[j].x, particles[j].y);
          ctx.strokeStyle = 'rgba(' + nodeR + ',' + nodeG + ',' + nodeB + ',' + alpha + ')';
          ctx.lineWidth = 0.7;
          ctx.stroke();
        }
      }
    }

    /* Draw nodes */
    for (var i = 0; i < particles.length; i++) {
      var p = particles[i];
      /* Subtle pulse */
      var pulse = 0.7 + 0.3 * Math.sin(p.pulse + t * 0.8);
      var r = p.r * pulse;
      ctx.beginPath();
      ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
      ctx.fillStyle = 'rgba(' + nodeR + ',' + nodeG + ',' + nodeB + ',' + (nodeAlpha * pulse) + ')';
      ctx.fill();
    }
  }

  /* Pause when tab is not visible */
  document.addEventListener('visibilitychange', function () {
    isVisible = !document.hidden;
  });

  window.addEventListener('resize', function () {
    resize();
    /* Re-scatter particles to fill new viewport */
    for (var i = 0; i < particles.length; i++) {
      if (particles[i].x > W || particles[i].y > H) {
        particles[i].x = Math.random() * W;
        particles[i].y = Math.random() * H;
      }
    }
    requestParallaxUpdate();
  });

  window.addEventListener('scroll', requestParallaxUpdate, { passive: true });

  /* Boot */
  init();
  applyParallax();
  requestAnimationFrame(tick);
})();
