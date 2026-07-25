# Verification: lib-game suggest/doc/deps findings (F9-F12, F15, F17-F20)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust @ f8763a5.
Verified by independent read of suggest.rs, doc.rs, Cargo.toml, parser/mod.rs,
game command files, repl.rs, notify.rs, plus grep. No code changes made.

### F9: suggest's Many arm ignores min/max
- original severity: minor
- verdict: CONFIRMED
- evidence: suggest.rs:109 `Spec::Many { spec, delim, .. }` destructures away
  `min`/`max`; the loop (111-144) never counts items and on item-parse failure
  falls through to `suggest_spec(spec, rem, names)` (line 141), offering a
  fresh item regardless of how many were consumed. Parse-side enforces max:
  parser/mod.rs:929-933 `if let Some(max_val) = max && values.len() >= *max_val { break; }`.
  Reachability confirmed: sushi-go-2 command.rs:42
  `Many::bounded_spaced(Int::bounded(1, max as i32), 1, 2)` and sushizock-2
  command.rs:47 `Many::bounded_spaced(Int::bounded(1, max as i32), 1, max)`;
  `Many::to_spec` (parser/mod.rs:415-422) propagates min/max into the Spec the
  suggest engine sees. Traced: `Many{max:2, delim:Space}` with input "1 2 "
  parses item, delim, item, delim, then inner parse of "" fails and line 141
  suggests a third item the real parser rejects. Severity minor is right.

### F10: Int suggestion range computation can overflow
- original severity: minor
- verdict: CONFIRMED
- evidence: suggest.rs:86-87
  `let start = min.unwrap_or(1); let end = max.map(|m| m.min(start + 4)).unwrap_or(start + 4);`
  with `min`/`max` being `Option<i32>` (command/mod.rs Spec::Int). `start + 4`
  overflows for `min >= i32::MAX - 3`: panic in debug, wrap to negative `end`
  in release (empty `start..=end` range, so silently no suggestions). Only
  spec-authors can trigger it, so minor/quality is fair; `saturating_add(4)`
  is the right fix.

### F11: doc_int renders open-ended minimum as 0
- original severity: minor
- verdict: CONFIRMED
- evidence: doc.rs:51 `(min, Some(max)) => vec![Node::text(format!("{}-{}", min.unwrap_or(0), max))]`
  catches `(None, Some(max))`, so `Int { min: None, max: Some(5) }` renders
  "0-5". The parser only rejects below-min when `min` is `Some`
  (parser/mod.rs:151-153), so negatives are accepted for this shape, and
  `Int::expected_output` (parser/mod.rs:113) says
  `(None, Some(max)) => format!("number {} or lower", max)` - the doc and the
  error text disagree. Reachability confirmed: for-sale-2 command.rs bid
  parser uses `Int { min: None, max: Some(max) }`; `spec.doc()` output reaches
  users via lib/cmd/src/repl.rs:95 (`doc::render(&spec.doc())`) and
  web/src/email/notify.rs:94 (`doc::render(&spec.doc())`). All claims check out.

### F12: doc_many drops a bounded max
- original severity: minor
- verdict: CONFIRMED
- evidence: doc.rs:134-142 - `(None, _) | (Some(0), _)` pushes `*` and
  `(Some(1), _)` pushes `+`; both arms match when `max` is `Some(n)` (n >= 2)
  because the only earlier Some-max arms are `(_, Some(0))`, min>max,
  `(Some(0)|None, Some(1))`, and `(Some(1), Some(1))`. So
  `Many { min: Some(1), max: Some(2) }` docs as `doc+`, losing the cap.
  One correction to the finding's framing: it calls this "latent - no game
  crate constructs Spec::Many directly", but `Many::to_spec`
  (parser/mod.rs:415-422) propagates min/max, and sushi-go-2 command.rs:42
  (`Many::bounded_spaced(.., 1, 2)`) plus sushizock-2 command.rs:47 feed
  `spec.doc()` via repl.rs:95 / notify.rs:94 - so the misrendering ("+"
  instead of "(1-2)") is reachable today, not latent. That strengthens the
  finding; the defect description and minor severity remain accurate.

### F15: `combine` declared but unused
- original severity: minor
- verdict: CONFIRMED
- evidence: lib/game/Cargo.toml:12 `combine = "4.6.7"`. `grep -rn "combine" src/`
  over lib/game/src returns no matches (exit 1). Sanity checks pass as the
  finding claims: `unicase` used at parser/mod.rs:5 (`use unicase::UniCase;`)
  and :51; `log` used at bot.rs:6 (`use ::log::trace;`); `serde_json` used in
  parser/mod.rs (CommandSpec impl) and also rng.rs. Dead dependency; remove.

### F17: case-folding differs between suggest and parse
- original severity: nit
- verdict: CONFIRMED
- evidence: suggest.rs:26 `token.to_lowercase().starts_with(&remaining.to_lowercase())`
  (also 36-39, 52-55, 98-101) and Enum::parse parser/mod.rs:605
  `input.to_lowercase()` vs Token::parse parser/mod.rs:51
  `UniCase::new(&input[..t_len]) != UniCase::new(&self.token)`. UniCase does
  Unicode caseless comparison with full case folding for non-ASCII (ss/ß
  compare equal), while `to_lowercase` prefix matching does not, so suggest
  filtering and Token acceptance can diverge for non-ASCII tokens. No in-tree
  spec uses non-ASCII tokens; nit is the right level.

### F18: suggestions not deduplicated
- original severity: nit
- verdict: CONFIRMED
- evidence: suggest.rs:37-44 maps `values.iter().filter(..).map(..)` verbatim
  with no dedupe; Enum::parse dedupes explicitly at parser/mod.rs:612-619
  (`let mut searched: HashSet<String> = HashSet::new(); ... if searched.contains(&v_str) { continue; }`).
  OneOf arm (suggest.rs:46-49) is a plain `flat_map` concat. Asymmetry with
  the parser is real; only degenerate specs hit it. Nit correct.

### F19: unbounded recursion over spec nesting
- original severity: nit
- verdict: CONFIRMED
- evidence: suggest_spec recurses in every composite arm (suggest.rs:48, 51,
  108, 118, 132, 141); spec parse recurses likewise (parser/mod.rs:907, 945).
  No depth guard anywhere. `Spec` is `#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]`
  (command/mod.rs:13-14), so a deeply nested spec is constructible via serde.
  Depth is spec-controlled, not user-input-controlled, and specs come from
  trusted game crates today - nit with "no action unless untrusted
  deserialization appears" is the right call.

### F20: Token("") yields empty suggestion and shadows later chain elements
- original severity: nit
- verdict: CONFIRMED
- evidence: suggest.rs:26 - with `token = ""` and `remaining = ""`,
  `"".starts_with("")` is true, so the Token arm returns
  `vec![Suggestion { value: "".into(), desc: None }]`. In the Chain arm
  (suggest.rs:73-76) `let suggs = suggest_spec(spec, rem, names); if !suggs.is_empty() { return suggs; }`
  - the vec containing one empty-string suggestion is non-empty, so the chain
  returns it immediately and never reaches the `spec.parse` advancement at
  line 77; later chain elements are never suggested. Note the shadowing only
  occurs when the remaining fragment is "" (for non-empty `rem`,
  `"".starts_with(rem)` is false, the Token arm returns `vec![]`, and
  `Token("").parse` succeeds zero-width so the chain advances normally).
  Only constructible via a degenerate spec; nit correct.
