# Email Rendering Guide

How outbound game emails are rendered, the Gmail hazard that can blank an
email's body, and a verification playbook for proving an email renders
before shipping it. Read this before touching anything under
`rust/web/src/email/` or debugging a "the email looks wrong" report.

Stack: `mrml` 6.0.1 (a Rust MJML parser/renderer), sent via Resend/SES.
Renderer: `rust/web/src/email/render.rs` (`render_game_email`). Board and
log markup: `brdgme_markup` (`html` for the themed HTML part, `plain` for
the unthemed text part). Theming: `brdgme_color` palettes.

## How email rendering works here

- `render_game_email` (`render.rs:86`) produces `{ subject, text, html,
  headers }` from generic `EmailContent` blocks (header, digest, board,
  you-can, browser link, rules link, footer) plus the recipient's palette.
- Each text block is `brdgme_markup` source. `render_block` runs it through
  `brdgme_markup::html` (concrete inline-styled `<span>`s) for the HTML part
  and `brdgme_markup::plain` for the text part (`render.rs:71`).
- The HTML body is assembled into ONE string and wrapped in a single `<pre>`
  (monospace, `white-space:pre-wrap`) so the ASCII board keeps its layout.
  That `<pre>` is injected into MJML chrome via `<mj-raw>` and rendered by
  `mrml` (`render.rs:170`). If `mrml` fails, `fallback_html` (`render.rs:74`)
  emits a bare `<pre>` in a `<body>`.
- Theming: `palette_for_slug` (`render.rs:46`) resolves the recipient's
  stored theme slug to a concrete palette; every colour is inlined as a hex
  value. Email clients cannot resolve `var(--...)` or see
  `prefers-color-scheme`, so concrete hex is mandatory.
- The text part is deliberately unthemed terminal output (`plain`), so it
  stays readable in any plain-text client.

## Rules to stay safe

1. **The board `<pre>` must sit in a real table cell and carry its own
   `font-size`.** The MJML at `render.rs:170` wraps the `<pre>` in
   `<tr><td style="padding:0;font-size:13px;">` inside the `<mj-raw>`, and
   the `<pre>` itself sets `font-size:13px`. Do not remove either. See the
   hazard below for why - this is the rule that keeps the email body
   visible in Gmail.
2. **Do not "fix" the wrapping by switching the board to `<mj-text>`.**
   `mj-text` re-parses its content and can collapse the significant
   whitespace the board depends on. `<mj-raw>` passes content through
   verbatim, which is what preserves the runs of spaces that draw the board.
   Keep `<mj-raw>`; supply the `<tr><td>` cell yourself.
3. **Any element that must stay visible no matter what should carry its own
   `font-size`** (the rules link sets `font-size:12px`, `render.rs:156`).
   This is defence in depth, not a substitute for rule 1.
4. **Concrete hex colours only** - never CSS custom properties.

## Known hazard: the `font-size:0` / foster-parenting collapse

`mrml` renders an `<mj-raw>` child directly into the column's `<tbody>` with
no `<tr><td>` wrapper (the raw-child branch of `mj_column`'s render loop). A
`<pre>` placed there is a direct child of `<tbody>`, which is invalid HTML.

Two things then happen, in any HTML5 parser:

- The parser **foster-parents** the `<pre>` out of the table, relocating it
  as a sibling before the table - into the MJML column wrapper `<div>`.
- That column wrapper `<div>` carries `font-size:0px` (MJML's standard trick
  for removing whitespace between inline-block columns).

A `<pre>` with no `font-size` of its own inherits that `0px`, so every glyph
inside it - the board, the header, the digest, the browser link, the footer -
collapses to zero height: invisible and non-selectable. The `<pre>`'s
`background-color` and `padding` still paint, leaving a dark box with no
readable content. Only an element that declares its own `font-size` (the
rules link, `font-size:12px`) escapes the collapse. That is the exact
"dark area with just a rules link" symptom.

A standards-compliant browser foster-parents and may still show the content
once the `<pre>` has a font-size, but Gmail's own HTML processing is NOT a
plain browser parse - it can drop the malformed table content outright. So
the fix is to emit VALID markup (the `<tr><td>` cell), not to rely on browser
error recovery.

## Verification playbook

A screenshot alone is not enough: the failure mode is `0px` glyphs, which
look like "nothing there" and are easy to misread. Measure computed styles
instead. The whole flow is offline and needs only `python3` and the nix
`chromium`.

1. **Get the raw email.** Gmail "Show original" -> save as `msg.eml`.

2. **Decode the quoted-printable HTML part** to a plain `.html` file:

   ```bash
   python3 - <<'PY'
   import email
   from email import policy
   msg = email.message_from_binary_file(open('msg.eml','rb'), policy=policy.default)
   for part in msg.walk():
       if part.get_content_type() == 'text/html':
           open('email.html','w').write(part.get_content())
           break
   PY
   ```

3. **Check the raw structure for the bug pattern.** In the DECODED html (not
   the browser DOM), the `<pre>` must not be a bare child of `<tbody>`:

   ```bash
   grep -c '<tbody><pre' email.html   # want 0; 1 means the bug is present
   ```

4. **Dump the parsed DOM** to see what a real HTML5 parser makes of it (this
   is where foster-parenting shows up - the `<pre>` relocated before the
   table, its parent the `font-size:0px` column div):

   ```bash
   chromium --headless --no-sandbox --disable-gpu --dump-dom \
     "file://$PWD/email.html" > dom.html
   ```

5. **Measure computed styles.** Inject a script that reports
   `getComputedStyle(...).fontSize` and `getBoundingClientRect().height` for
   the `<pre>`, the links, and a board cell, then read the report back via
   `--dump-dom`. A healthy email shows the `<pre>` at a non-zero font-size
   with a content-sized height; the bug shows `0px` and a padding-only
   height (~32-46px):

   ```bash
   python3 - <<'PY'
   html = open('email.html').read()
   script = """
   <script>
   function fs(el){ return el ? getComputedStyle(el).fontSize : 'MISSING'; }
   function box(el){ return el ? el.getBoundingClientRect().height.toFixed(1) : 'MISSING'; }
   window.addEventListener('load', function(){
     var pre = document.querySelector('pre');
     var out = ['PRE_font_size=' + fs(pre), 'PRE_height=' + box(pre),
       'PRE_parent=' + (pre&&pre.parentElement?pre.parentElement.tagName:'none')];
     document.querySelectorAll('a').forEach(function(a){
       out.push('LINK[' + a.textContent.trim() + ']=' + fs(a) + '/' + box(a)); });
     var d=document.createElement('div'); d.id='REPORT';
     d.setAttribute('style','font-size:14px;');
     d.textContent=out.join(' || '); document.body.appendChild(d);
   });
   </script>
   """
   open('measure.html','w').write(html.replace('</body>', script + '</body>'))
   PY
   chromium --headless --no-sandbox --disable-gpu --virtual-time-budget=2000 \
     --dump-dom "file://$PWD/measure.html" 2>/dev/null \
     | grep -o 'id="REPORT"[^>]*>.*</div>'
   ```

6. **Iterate on the markup without rebuilding `web`.** Rendering the MJML
   only needs `mrml`, so a throwaway crate that depends on `mrml = "=6.0.1"`
   and calls `mrml::parse(...).element.render(...)` reproduces the exact
   HTML in seconds - far faster than compiling the leptos/sqlx `web` crate.

7. **Regression test.** `render.rs:329`
   (`render_game_email_html_pre_is_valid_table_content_with_font_size`)
   asserts the produced HTML has no `<tbody><pre` and that the `<pre>`
   declares a `font-size`. Run it with the offline sqlx cache:
   `SQLX_OFFLINE=true cargo test -p web --features ssr email::render`.

## Case history: the 2026-07-21 sparrowhawk incident

A turn notification for game `4de6c76a-4ece-430f-b584-b2f46dcdd954`
("Acquire with sparrowhawk") rendered in Gmail as a dark box containing only
the "View rules" link - no board, no header, no other text, nothing else
selectable. The recipient's dark theme was correctly applied (the dark
background was there), so theming was not at fault.

Root cause: the board `<pre>` was emitted via `<mj-raw>` as a bare child of
`<tbody>` (invalid HTML). Gmail foster-parented it into the column's
`font-size:0px` wrapper div; the `<pre>` had no font-size of its own, so all
its glyphs collapsed to `0px`. The rules link survived only because it sets
`font-size:12px` explicitly. Confirmed by decoding the `.eml`, dumping the
parsed DOM (the `<pre>` foster-parented under the `font-size:0px` div), and
measuring computed styles (`<pre>` at `0px`, height `46px` of padding only;
every element `0px` except the rules link).

Fix (`render.rs:170`): wrap the `<pre>` in `<tr><td style="padding:0;
font-size:13px;">` inside the `<mj-raw>` (valid table markup, so no
foster-parenting) and give the `<pre>` its own `font-size:13px` (defence in
depth). Verified the same way: `<pre>` now parented by a `<td>` at `13px`,
height ~122px of real content, both links and the board cells visible. The
regression test at `render.rs:329` locks the structure in.
