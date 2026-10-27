// On a phone or tablet (a touch screen and no mouse, or a very small screen), says up
// front that Instance is made for a computer: the IDE needs a mouse and room, and most
// games need a keyboard. A classic dialog over the page, until "Continue anyway"
// (remembered for the browser tab).
(function () {
  var mm = function (q) { return window.matchMedia && window.matchMedia(q).matches; };
  var touchOnly = mm('(pointer: coarse)') && !mm('(any-pointer: fine)');
  var tiny = Math.min(screen.width, screen.height) < 500;
  if (!touchOnly && !tiny) return;
  try { if (sessionStorage.getItem('instance-phone-ok')) return; } catch (e) {}

  var css =
    '#phone-note{position:fixed;inset:0;z-index:5000;display:flex;align-items:center;justify-content:center;' +
    'padding:16px;background:rgba(0,0,0,.6);font:14px/1.45 "Segoe UI",Tahoma,system-ui,sans-serif;color:#0a0a0a}' +
    '#phone-note .win{box-sizing:border-box;width:100%;max-width:380px;padding:3px;background:#f0f0f0;' +
    'border:1px solid;border-color:#fff #696969 #696969 #fff;box-shadow:inset 1px 1px #fff,inset -1px -1px #a0a0a0}' +
    '#phone-note .cap{display:flex;align-items:center;gap:5px;height:22px;padding:0 4px;' +
    'background:linear-gradient(90deg,#0a246a,#a6caf0);color:#fff;font-weight:700}' +
    '#phone-note .cap img{width:16px;height:16px}' +
    '#phone-note .body{display:flex;gap:12px;padding:14px 12px 6px}' +
    '#phone-note .icon{flex:none;width:32px;height:32px;border-radius:50%;background:#1c5fc8;color:#fff;' +
    'font:700 20px/32px Georgia,serif;text-align:center}' +
    '#phone-note p{margin:0 0 8px}' +
    '#phone-note .acts{display:flex;justify-content:flex-end;padding:4px 12px 12px}' +
    '#phone-note button{min-width:120px;padding:6px 14px;background:#f0f0f0;color:#0a0a0a;font:inherit;cursor:pointer;' +
    'border:1px solid;border-color:#fff #696969 #696969 #fff;box-shadow:inset -1px -1px #a0a0a0;outline:1px solid #000}' +
    '#phone-note button:active{border-color:#696969 #fff #fff #696969;box-shadow:inset 1px 1px #a0a0a0}';

  function show() {
    var style = document.createElement('style');
    style.textContent = css;
    document.head.appendChild(style);
    var d = document.createElement('div');
    d.id = 'phone-note';
    d.setAttribute('role', 'alertdialog');
    d.setAttribute('aria-modal', 'true');
    d.setAttribute('aria-labelledby', 'phone-note-title');
    d.innerHTML =
      '<div class="win"><div class="cap"><img src="favicon.ico" alt=""><span id="phone-note-title">' +
      'Made for a computer</span></div>' +
      '<div class="body"><div class="icon" aria-hidden="true">i</div><div>' +
      '<p><b>Instance was not built for phones or tablets.</b></p>' +
      '<p>It is a desktop game maker: the editor needs a mouse and a big screen, and most ' +
      'games need a keyboard to play. On a touch screen, much of it will not work.</p>' +
      '<p>For the real thing, open this page on a computer.</p>' +
      '</div></div><div class="acts"><button type="button">Continue anyway</button></div></div>';
    document.body.appendChild(d);
    var b = d.querySelector('button');
    b.addEventListener('click', function () {
      try { sessionStorage.setItem('instance-phone-ok', '1'); } catch (e) {}
      d.remove();
    });
    b.focus();
  }
  if (document.body) show(); else document.addEventListener('DOMContentLoaded', show);
})();
