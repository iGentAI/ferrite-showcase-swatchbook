//! Static CSS and JavaScript, inlined into the pages.
//!
//! `BASE_CSS` styles every layout through the theme's custom properties
//! (`--bg`, `--surface`, `--ink`, `--muted`, `--accent`, `--accent-ink`,
//! `--radius`, `--scale`, `--font-display`, `--font-body`). The three layouts
//! are selected by a `layout-*` class on `<body>` and share the same markup,
//! so switching design never changes the content.

pub const BASE_CSS: &str = r##"
:root{--gutter:clamp(16px,4vw,48px)}
*,*::before,*::after{box-sizing:border-box}
*{margin:0;padding:0}
html{font-size:calc(16px * var(--scale,1));-webkit-text-size-adjust:100%}
body{min-height:100vh;background:var(--bg);color:var(--ink);font-family:var(--font-body);line-height:1.5;-webkit-font-smoothing:antialiased;padding-bottom:110px;transition:background-color .25s,color .25s}
a{color:inherit}
button,input,select,textarea{font:inherit;color:inherit}
button{cursor:pointer}
.masthead{display:flex;align-items:center;gap:14px;flex-wrap:wrap;padding:18px var(--gutter)}
.brand{display:inline-flex;align-items:center;gap:10px;text-decoration:none;font-family:var(--font-display);font-weight:800;font-size:1.2rem;letter-spacing:-.01em}
.logo{width:22px;height:22px;border-radius:calc(var(--radius) / 2);background:conic-gradient(var(--accent) 0 25%,var(--ink) 0 50%,var(--muted) 0 75%,var(--surface) 0);box-shadow:0 0 0 1px var(--ink)}
.design-tag{font-size:.72rem;text-transform:uppercase;letter-spacing:.12em;color:var(--muted);border:1px solid currentColor;border-radius:999px;padding:3px 10px;white-space:nowrap}
.spacer{flex:1}
.btn{display:inline-flex;align-items:center;gap:6px;border:0;background:var(--accent);color:var(--accent-ink);padding:10px 16px;border-radius:var(--radius);font-weight:700;line-height:1.1;text-decoration:none}
.btn:hover{filter:brightness(1.06)}
.btn:focus-visible,.like:focus-visible,.del:focus-visible{outline:3px solid var(--accent);outline-offset:2px}
.btn.ghost{background:transparent;color:var(--ink);box-shadow:inset 0 0 0 1px var(--ink)}
.btn[disabled]{opacity:.6;cursor:progress}
.intro{padding:20px var(--gutter) 6px}
.intro h1{font-family:var(--font-display);font-size:clamp(2.1rem,5.5vw,4rem);line-height:1.02;letter-spacing:-.025em;max-width:16ch}
.count{margin-top:12px;color:var(--muted);font-size:.95rem}
.count b{color:var(--ink)}
.pals{padding:26px var(--gutter) 56px}
.pal{position:relative}
.sw{display:flex}
.c{flex:1 1 0;min-width:0;background:var(--c);display:flex;align-items:flex-end}
.c i{font-style:normal;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;font-size:.66rem;letter-spacing:.04em;padding:6px;opacity:0;transition:opacity .15s}
.c.on-dark i{color:#fff}
.c.on-light i{color:#111}
.pal:hover .c i{opacity:1}
.meta{display:flex;align-items:center;gap:12px}
.text{min-width:0}
.pname{font-family:var(--font-display);font-size:1.05rem;line-height:1.2;overflow-wrap:anywhere}
.by{color:var(--muted);font-size:.8rem}
.idx{display:none}
.like{margin-left:auto;flex:none;display:inline-flex;align-items:center;gap:6px;background:transparent;border:1px solid color-mix(in srgb,var(--ink) 22%,transparent);color:var(--ink);padding:5px 11px;border-radius:999px;font-size:.85rem;font-variant-numeric:tabular-nums;transition:background-color .15s,color .15s,border-color .15s}
.like:hover{border-color:var(--accent)}
.like.liked{background:var(--accent);border-color:var(--accent);color:var(--accent-ink)}
.heart{font-size:.95rem;line-height:1}
.del{display:none;flex:none;border:0;background:transparent;color:var(--muted);font-size:1.25rem;line-height:1;padding:2px 4px}
.admin .del{display:inline-block}
.empty{color:var(--muted);padding:40px 0}
.votebar{position:fixed;left:0;right:0;bottom:0;z-index:20;display:flex;align-items:center;flex-wrap:wrap;gap:10px 16px;padding:12px var(--gutter);background:var(--ink);color:var(--bg);font-size:.88rem}
.votebar .btn{padding:8px 14px}
.votebar .btn.voted{background:transparent;color:var(--bg);box-shadow:inset 0 0 0 1px var(--bg);cursor:default}
.score{font-variant-numeric:tabular-nums}
.vb-links{margin-left:auto;display:flex;gap:16px;flex-wrap:wrap}
.vb-links a{color:var(--bg);opacity:.85;text-underline-offset:3px}
.vb-links a:hover{opacity:1}
.embed [data-open-add],.embed .vb-links{display:none}
dialog{margin:auto;border:0;border-radius:max(var(--radius),6px);padding:26px;width:min(520px,calc(100vw - 32px));background:var(--surface);color:var(--ink);box-shadow:0 30px 80px rgba(0,0,0,.35)}
dialog::backdrop{background:rgba(10,10,10,.5)}
dialog h2{font-family:var(--font-display);font-size:1.5rem;margin-bottom:6px}
.field{display:block;font-size:.8rem;color:var(--muted);margin:14px 0 6px}
dialog input[type=text]{width:100%;padding:10px 12px;border-radius:calc(var(--radius) * .6);border:1px solid color-mix(in srgb,var(--ink) 25%,transparent);background:var(--bg);color:var(--ink)}
.colors{display:flex;gap:8px;flex-wrap:wrap}
.colors input[type=color]{width:56px;height:56px;border:0;padding:0;background:none;cursor:pointer;border-radius:8px}
.color-tools{display:flex;gap:8px;margin-top:10px}
.color-tools .btn{padding:6px 10px;font-size:.8rem}
.err{color:#d11;font-size:.85rem;min-height:1.3em;margin-top:10px}
.form-actions{display:flex;justify-content:flex-end;gap:10px;margin-top:8px}

/* Layout: gallery grid */
.layout-gallery .pals{display:grid;grid-template-columns:repeat(auto-fill,minmax(250px,1fr));gap:20px}
.layout-gallery .pal{background:var(--surface);border-radius:var(--radius);overflow:hidden;box-shadow:0 0 0 1px color-mix(in srgb,var(--ink) 8%,transparent),0 1px 2px color-mix(in srgb,var(--ink) 6%,transparent);transition:transform .2s,box-shadow .2s}
.layout-gallery .pal:hover{transform:translateY(-2px);box-shadow:0 0 0 1px color-mix(in srgb,var(--ink) 10%,transparent),0 14px 30px -14px color-mix(in srgb,var(--ink) 35%,transparent)}
.layout-gallery .sw{height:150px}
.layout-gallery .meta{padding:14px 16px}

/* Layout: poster bands */
.layout-poster .masthead{border-bottom:3px solid var(--ink)}
.layout-poster .brand{text-transform:uppercase;letter-spacing:.02em}
.layout-poster .design-tag{border-radius:0}
.layout-poster .btn{border-radius:0;text-transform:uppercase;letter-spacing:.08em}
.layout-poster .intro{padding-top:36px}
.layout-poster .intro h1{text-transform:uppercase;font-size:clamp(2.8rem,9vw,7.5rem);line-height:.96;letter-spacing:-.02em;max-width:none}
.layout-poster .count{text-transform:uppercase;letter-spacing:.1em;font-size:.8rem}
.layout-poster .pals{display:flex;flex-direction:column;padding-left:0;padding-right:0}
.layout-poster .pal{display:grid;grid-template-columns:minmax(240px,36%) 1fr;min-height:190px;border-top:3px solid var(--ink)}
.layout-poster .pal:last-child{border-bottom:3px solid var(--ink)}
.layout-poster .meta{order:-1;flex-direction:column;align-items:flex-start;justify-content:space-between;gap:14px;padding:20px var(--gutter);border-right:3px solid var(--ink)}
.layout-poster .idx{display:block;color:var(--accent);font-size:.85rem;letter-spacing:.1em}
.layout-poster .pname{font-size:clamp(1.6rem,3.4vw,2.9rem);text-transform:uppercase;line-height:.92}
.layout-poster .by{text-transform:uppercase;letter-spacing:.1em;font-size:.72rem}
.layout-poster .like{margin-left:0;border-radius:0;border:2px solid var(--ink)}
.layout-poster .like.liked{border-color:var(--accent)}
.layout-poster .c i{opacity:1;writing-mode:vertical-rl;transform:rotate(180deg);padding:14px 10px;font-size:.72rem}
.layout-poster .votebar{border-top:3px solid var(--accent)}
.layout-poster dialog{border-radius:0}
@media (max-width:720px){.layout-poster .pal{grid-template-columns:1fr}.layout-poster .meta{border-right:0}.layout-poster .sw{height:120px}}

/* Layout: soft cards */
.layout-soft .intro h1{font-style:italic;font-weight:700}
.layout-soft .pals{columns:290px;column-gap:22px}
.layout-soft .pal{break-inside:avoid;display:inline-block;width:100%;margin-bottom:22px;background:var(--surface);border-radius:var(--radius);padding:22px 22px 18px;box-shadow:0 18px 40px -22px color-mix(in srgb,var(--accent) 55%,transparent)}
.layout-soft .sw{align-items:center;padding:4px 0 18px 14px}
.layout-soft .c{flex:0 1 72px;height:auto;aspect-ratio:1;border-radius:50%;margin-left:-14px;box-shadow:0 0 0 4px var(--surface),0 4px 10px -4px rgba(0,0,0,.25)}
.layout-soft .c i{display:none}
.layout-soft .like{border:0;background:color-mix(in srgb,var(--accent) 14%,transparent)}
.layout-soft .like.liked{background:var(--accent)}
.layout-soft .design-tag{border:0;background:color-mix(in srgb,var(--accent) 14%,transparent);color:var(--ink)}
.layout-soft .votebar{left:12px;right:12px;bottom:12px;border-radius:var(--radius)}
"##;

pub const STUDIO_CSS: &str = r##"
body.studio{padding-right:360px}
.studio .votebar{right:360px}
.studio-panel{position:fixed;top:12px;right:12px;bottom:12px;width:336px;overflow:auto;z-index:40;background:#fbfbfa;color:#18181b;border-radius:14px;box-shadow:0 24px 60px rgba(0,0,0,.28),0 0 0 1px rgba(0,0,0,.08);padding:18px 18px 24px;font:13px/1.45 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif}
.studio-panel h2{font-size:16px;margin-bottom:4px}
.studio-panel h3{font-size:11px;text-transform:uppercase;letter-spacing:.1em;color:#71717a;margin:18px 0 8px}
.studio-panel .hint{color:#52525b}
.studio-panel .hint.warn{color:#9a3412;margin-top:8px}
.studio-panel label{display:flex;align-items:center;justify-content:space-between;gap:10px;margin:6px 0}
.studio-panel label.stack{flex-direction:column;align-items:stretch;gap:4px}
.studio-panel input[type=text],.studio-panel input[type=password],.studio-panel input[type=url],.studio-panel select,.studio-panel textarea{width:100%;padding:7px 9px;border:1px solid #d4d4d8;border-radius:8px;background:#fff;color:#18181b;font:inherit}
.studio-panel textarea{font-family:ui-monospace,Menlo,monospace;font-size:12px;min-height:110px;resize:vertical}
.studio-panel input[type=color]{width:44px;height:28px;border:1px solid #d4d4d8;border-radius:6px;padding:0;background:#fff;cursor:pointer}
.studio-panel input[type=range]{flex:1}
.studio-panel output{min-width:44px;text-align:right;font-variant-numeric:tabular-nums;color:#52525b}
.scores{display:grid;grid-template-columns:repeat(4,1fr);gap:6px}
.scores div{background:#fff;border:1px solid #e4e4e7;border-radius:8px;padding:6px 8px}
.scores dt{font-size:10px;color:#71717a;text-transform:uppercase;letter-spacing:.08em}
.scores dd{font-size:16px;font-weight:700;font-variant-numeric:tabular-nums}
.presets{display:grid;grid-template-columns:repeat(3,1fr);gap:8px}
.preset{border:1px solid #d4d4d8;border-radius:10px;background:#fff;padding:8px;text-align:left;font:600 12px/1.2 system-ui,sans-serif;color:#18181b}
.preset span{display:block;height:26px;border-radius:6px;margin-bottom:6px;box-shadow:inset 0 0 0 1px rgba(0,0,0,.08)}
.preset:hover{border-color:#18181b}
.sbtn{display:block;width:100%;margin-top:8px;padding:9px 12px;border-radius:9px;border:1px solid #d4d4d8;background:#fff;color:#18181b;font:600 13px/1.2 system-ui,sans-serif}
.sbtn:hover{border-color:#18181b}
.sbtn.primary{background:#18181b;color:#fff;border-color:#18181b}
.sbtn.danger{color:#b91c1c}
.status{min-height:1.4em;margin-top:8px;color:#15803d}
.status.bad{color:#b91c1c}
.publish-bar{position:sticky;bottom:0;z-index:1;margin:8px -18px 0;padding:4px 18px 12px;background:#fbfbfa;border-top:1px solid #e4e4e7;box-shadow:0 -10px 16px -12px rgba(0,0,0,.18)}
@media (max-width:960px){body.studio{padding-right:0}.studio .votebar{right:0}.studio-panel{position:static;width:auto;margin:12px}}
"##;

pub const COMPARE_CSS: &str = r##"
*{box-sizing:border-box;margin:0;padding:0}
body{font:15px/1.45 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif;background:#111;color:#f4f4f5;min-height:100vh;display:flex;flex-direction:column}
.cmp-head{display:flex;flex-wrap:wrap;align-items:baseline;gap:6px 18px;padding:16px 20px}
.cmp-head b{font-size:18px}
.cmp-head span{color:#a1a1aa}
.frames{flex:1;display:grid;grid-template-columns:1fr 1fr;gap:12px;padding:0 12px 12px}
.frames figure{display:flex;flex-direction:column;min-height:78vh;background:#18181b;border-radius:12px;overflow:hidden}
figcaption{display:flex;justify-content:space-between;gap:10px;padding:10px 14px;font-size:13px;color:#d4d4d8}
figcaption a,.note a,.empty a{color:#fafafa}
iframe{flex:1;width:100%;border:0;background:#fff}
.placeholder .empty{flex:1;display:flex;flex-direction:column;justify-content:center;gap:10px;padding:32px;color:#a1a1aa;max-width:46ch;margin:0 auto}
.note{padding:0 20px 16px;color:#a1a1aa;font-size:13px}
@media (max-width:900px){.frames{grid-template-columns:1fr}.frames figure{min-height:70vh}}
"##;

/// Gallery behaviour: visitor id, likes, design votes, palette submission
/// and (for admins) removal. Shared by the gallery and the Studio.
pub const APP_JS: &str = r##"
(function(){
  'use strict';
  var body = document.body;
  function uid(){
    if (window.crypto && crypto.randomUUID) return crypto.randomUUID();
    var a = new Uint8Array(16); crypto.getRandomValues(a);
    return Array.prototype.map.call(a, function(b){ return ('0' + b.toString(16)).slice(-2); }).join('');
  }
  function store(key, value){
    try {
      if (value === undefined) return localStorage.getItem(key);
      if (value === null) localStorage.removeItem(key); else localStorage.setItem(key, value);
    } catch (e) { return null; }
    return value;
  }
  var voter = store('sb-voter');
  if (!voter || !/^[A-Za-z0-9-]{8,64}$/.test(voter)) { voter = uid(); store('sb-voter', voter); }
  function api(path, payload, key){
    var headers = {'content-type': 'application/json'};
    if (key) headers['idempotency-key'] = key;
    return fetch(path, {method: 'POST', headers: headers, body: JSON.stringify(payload || {})}).then(function(r){
      return r.json().catch(function(){ return {}; }).then(function(j){
        if (!r.ok) throw new Error(j.error || ('Request failed (' + r.status + ')'));
        return j;
      });
    });
  }
  api('/api/visit', {voter: voter}).catch(function(){});

  var liked = {};
  try { liked = JSON.parse(store('sb-liked') || '{}') || {}; } catch (e) { liked = {}; }
  document.querySelectorAll('.like').forEach(function(b){ if (liked[b.dataset.id]) b.classList.add('liked'); });
  if (store('sb-admin')) body.classList.add('admin');

  document.addEventListener('click', function(e){
    var like = e.target.closest('.like');
    if (like) {
      if (like.classList.contains('liked')) return;
      var id = like.dataset.id;
      like.classList.add('liked');
      api('/api/palettes/' + encodeURIComponent(id) + '/like', {voter: voter}).then(function(j){
        like.querySelector('b').textContent = j.likes;
        liked[id] = 1; store('sb-liked', JSON.stringify(liked));
      }).catch(function(){ like.classList.remove('liked'); });
      return;
    }
    var del = e.target.closest('.del');
    if (del) {
      if (!confirm('Remove this palette for everyone on this design?')) return;
      api('/api/palettes/' + encodeURIComponent(del.dataset.id) + '/delete', {admin_key: store('sb-admin') || ''}).then(function(){
        var card = del.closest('.pal'); if (card) card.remove();
      }).catch(function(err){ alert(err.message); });
    }
  });

  var bar = document.querySelector('.votebar');
  var voteBtn = document.getElementById('vote');
  if (bar && voteBtn) {
    var votedKey = 'sb-voted-' + bar.dataset.round;
    function markVoted(){ voteBtn.classList.add('voted'); voteBtn.textContent = 'Thanks for voting'; }
    if (store(votedKey)) markVoted();
    voteBtn.addEventListener('click', function(){
      if (voteBtn.classList.contains('voted')) return;
      voteBtn.disabled = true;
      api('/api/vote', {voter: voter}).then(function(j){
        document.getElementById('votes').textContent = j.votes;
        store(votedKey, '1'); markVoted();
      }).catch(function(err){ alert(err.message); }).then(function(){ voteBtn.disabled = false; });
    });
  }

  var dlg = document.getElementById('add-dialog');
  if (dlg) {
    var form = document.getElementById('add-form');
    var colors = document.getElementById('f-colors');
    var err = document.getElementById('f-err');
    var submit = document.getElementById('f-submit');
    var submitKey = null;
    function hsl(h, s, l){
      s /= 100; l /= 100;
      var a = s * Math.min(l, 1 - l);
      function f(n){ var k = (n + h / 30) % 12; return l - a * Math.max(-1, Math.min(k - 3, Math.min(9 - k, 1))); }
      return '#' + [f(0), f(8), f(4)].map(function(x){ return ('0' + Math.round(x * 255).toString(16)).slice(-2); }).join('');
    }
    function addSwatch(value){
      if (colors.children.length >= 6) return;
      var input = document.createElement('input');
      input.type = 'color';
      input.value = value || hsl(Math.random() * 360, 60, 55);
      input.setAttribute('aria-label', 'Colour ' + (colors.children.length + 1));
      colors.appendChild(input);
    }
    function reset(){
      form.reset();
      colors.innerHTML = '';
      var base = Math.random() * 360;
      for (var k = 0; k < 5; k++) addSwatch(hsl((base + k * 28) % 360, 55 + Math.random() * 25, 28 + k * 13));
      err.textContent = '';
      submit.disabled = false;
      submitKey = uid();
    }
    document.querySelectorAll('[data-open-add]').forEach(function(b){
      b.addEventListener('click', function(){ reset(); if (dlg.showModal) dlg.showModal(); else dlg.setAttribute('open', ''); });
    });
    document.getElementById('f-more').addEventListener('click', function(){ addSwatch(); });
    document.getElementById('f-less').addEventListener('click', function(){ if (colors.children.length > 3) colors.lastElementChild.remove(); });
    document.getElementById('f-cancel').addEventListener('click', function(){ dlg.close(); });
    form.addEventListener('submit', function(e){
      e.preventDefault();
      var payload = {
        name: form.elements.name.value,
        author: form.elements.author.value,
        colors: Array.prototype.map.call(colors.querySelectorAll('input'), function(i){ return i.value; })
      };
      submit.disabled = true; err.textContent = '';
      api('/api/palettes', payload, submitKey).then(function(){ location.reload(); })
        .catch(function(x){ err.textContent = x.message; submit.disabled = false; });
    });
  }
})();
"##;

/// Studio behaviour: live preview of every control, presets, publish, and
/// the variant settings. Reads its initial state from `#sb-boot`.
pub const STUDIO_JS: &str = r##"
(function(){
  'use strict';
  var boot = JSON.parse(document.getElementById('sb-boot').textContent);
  var presets = boot.catalog.presets, fonts = boot.catalog.fonts;
  var colorKeys = ['bg', 'surface', 'ink', 'muted', 'accent'];
  function $(id){ return document.getElementById(id); }
  function lin(c){ c /= 255; return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4); }
  function inkFor(hex){
    var n = parseInt(hex.slice(1), 16);
    var L = 0.2126 * lin((n >> 16) & 255) + 0.7152 * lin((n >> 8) & 255) + 0.0722 * lin(n & 255);
    return (1.05 / (L + 0.05)) > ((L + 0.05) / 0.05) ? '#ffffff' : '#111111';
  }
  function fontsHref(t){
    var families = [];
    [t.font_display, t.font_body].forEach(function(k){
      var f = fonts[k];
      if (f && f.google && families.indexOf(f.google) < 0) families.push(f.google);
    });
    if (!families.length) return '';
    return 'https://fonts.googleapis.com/css2?' + families.map(function(f){ return 'family=' + f; }).join('&') + '&display=swap';
  }
  function read(){
    var t = {
      name: $('t-name').value, layout: $('t-layout').value,
      font_display: $('t-fd').value, font_body: $('t-fb').value,
      radius: parseInt($('t-radius').value, 10), scale: parseInt($('t-scale').value, 10),
      custom_css: $('t-css').value
    };
    colorKeys.forEach(function(k){ t[k] = $('t-' + k).value; });
    return t;
  }
  function fill(t){
    $('t-name').value = t.name; $('t-layout').value = t.layout;
    $('t-fd').value = t.font_display; $('t-fb').value = t.font_body;
    $('t-radius').value = t.radius; $('t-scale').value = t.scale;
    $('t-css').value = t.custom_css || '';
    colorKeys.forEach(function(k){ $('t-' + k).value = t[k]; });
  }
  function apply(t){
    var st = document.documentElement.style;
    colorKeys.forEach(function(k){ st.setProperty('--' + k, t[k]); });
    st.setProperty('--accent-ink', inkFor(t.accent));
    st.setProperty('--radius', t.radius + 'px');
    st.setProperty('--scale', (t.scale / 100).toFixed(2));
    if (fonts[t.font_display]) st.setProperty('--font-display', fonts[t.font_display].stack);
    if (fonts[t.font_body]) st.setProperty('--font-body', fonts[t.font_body].stack);
    document.body.className = document.body.className.replace(/\blayout-[a-z]+\b/, 'layout-' + t.layout);
    $('sb-custom').textContent = t.custom_css || '';
    var href = fontsHref(t), link = $('sb-fonts');
    if (href && link.getAttribute('href') !== href) link.setAttribute('href', href);
    document.querySelectorAll('.design-name').forEach(function(el){ el.textContent = t.name; });
    $('o-radius').textContent = t.radius + 'px';
    $('o-scale').textContent = t.scale + '%';
  }
  function update(){ apply(read()); }
  function status(id, message, bad){ var el = $(id); el.textContent = message; el.classList.toggle('bad', !!bad); }
  function post(path, payload){
    return fetch(path, {method: 'POST', headers: {'content-type': 'application/json'}, body: JSON.stringify(payload)}).then(function(r){
      return r.json().catch(function(){ return {}; }).then(function(j){
        if (!r.ok) throw new Error(j.error || ('Request failed (' + r.status + ')'));
        return j;
      });
    });
  }
  var panel = document.querySelector('.studio-panel');
  panel.addEventListener('input', function(e){ if (e.target.hasAttribute('data-theme')) update(); });
  panel.addEventListener('change', function(e){ if (e.target.hasAttribute('data-theme')) update(); });
  panel.querySelectorAll('.preset').forEach(function(b){
    b.addEventListener('click', function(){
      var p = presets[b.dataset.preset];
      if (!p) return;
      fill(p); update();
      status('t-status', 'Preset "' + p.name + '" loaded. Publish to make it live.');
    });
  });
  var keyInput = $('t-key');
  try { keyInput.value = localStorage.getItem('sb-admin') || ''; } catch (e) {}
  function adminKey(){
    var k = keyInput.value.trim();
    try { if (k) { localStorage.setItem('sb-admin', k); document.body.classList.add('admin'); } } catch (e) {}
    return k;
  }
  keyInput.addEventListener('change', adminKey);
  $('t-revert').addEventListener('click', function(){ fill(boot.theme); update(); status('t-status', 'Back to the live design.'); });
  $('t-publish').addEventListener('click', function(){
    status('t-status', 'Publishing...');
    post('/api/theme', {admin_key: adminKey(), theme: read()}).then(function(j){
      boot.theme = j.theme; fill(j.theme); apply(j.theme);
      status('t-status', 'Published. Everyone on this variant now sees "' + j.theme.name + '".');
    }).catch(function(e){ status('t-status', e.message, true); });
  });
  var claim = $('t-claim');
  if (claim) {
    claim.addEventListener('click', function(){
      post('/api/admin/claim', {admin_key: adminKey()}).then(function(){
        status('t-status', 'Claimed. This key now publishes designs for this app.');
        claim.remove();
      }).catch(function(e){ status('t-status', e.message, true); });
    });
  }
  $('s-save').addEventListener('click', function(){
    var site = {label: $('s-label').value, title: $('s-title').value, tagline: $('s-tagline').value, sibling_url: $('s-sibling').value};
    status('s-status', 'Saving...');
    post('/api/site', {admin_key: adminKey(), site: site}).then(function(j){
      document.querySelectorAll('.variant-label').forEach(function(el){ el.textContent = j.site.label; });
      $('s-sibling').value = j.site.sibling_url;
      status('s-status', 'Saved.');
    }).catch(function(e){ status('s-status', e.message, true); });
  });
  $('s-reset').addEventListener('click', function(){
    if (!confirm('Start a new scoreboard round? Visitors, votes, likes and additions start again from zero.')) return;
    post('/api/admin/reset-scores', {admin_key: adminKey()}).then(function(j){
      status('s-status', 'Round ' + j.round + ' started. Reloading...');
      // Reload so the vote button, its per-round marker and the scoreboard
      // all pick up the new round.
      setTimeout(function(){ location.reload(); }, 600);
    }).catch(function(e){ status('s-status', e.message, true); });
  });
  update();
})();
"##;
