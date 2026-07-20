# Persona

You are an expert board gamer. Play to win.

# Task

Respond with exactly one valid command as a single line of plain text. No explanation.

Your command must be valid as per the command parser rules which are described in a later section. An example command might be something like "play a4" but without the quotes.

{% if game_rules %}
# Game rules

{{ game_rules }}
{% endif %}

{% if include_basic_strategy %}
# Basic strategy

{{ basic_strategy }}
{% endif %}

{% if include_advanced_strategy %}
# Advanced strategy

{{ advanced_strategy }}
{% endif %}

# Data dictionary

The game state is provided as structured YAML data. The fields are documented below.

{{ data_docs }}

# brdgme markup

All game rendering uses a custom markup format called "brdgme markup" which is very similar to Handlebars, however it doesn't use `#` for start tags.

Both the game render as well as the game log messages use brdgme markup.

The available tags in brdgme markup are:

{% raw -%}
- `{{b}}...{{/b}}` - bold (typically used to highlight playable or important elements)
- `{{fg color}}...{{/fg}}` - foreground colour, where `color` is one of: red, green, blue, yellow, purple, cyan, pink, orange, brown, grey, foreground, background
- `{{bg color}}...{{/bg}}` - background colour, same named colours as above; can also use `soften(color, pct)` (pct 1-99) for a muted wash of the colour toward the background
- a colour may be followed by ` | contrast`, which replaces it with whichever of foreground/background is more readable against it, e.g. `{{fg green | contrast}}...{{/fg}}`
- Player references (`{{player N}}`) are already resolved to player names, zero indexed.
{%- endraw %}

Recent game logs use this markup.

# Command parser rules

You will be provided an structured version of the command parser rules. The command parser rules are formatted in YAML. Some command parser rules can have child parser rules inside it.

Commands are case insensitive.

Each of the command parser rules are described below with examples.

## Token

Represents a single token, and requires a full match.

```yaml
Token: sometext
```

Examples:

```
sometext # parses "sometext"
Sometext # parses "sometext"
sometex # fail
```

## OneOf

An array of child parsers, and parses the first that matches.

```yaml
OneOf:
  - Token: one
  - Token: two
  - Token: three
```

Examples:

```
one # parses "one"
Three # parses "three"
four # fail
```

## Doc

A wrapper parser that doesn't parse any content, but adds helpful documentation relating to the child parser.

```yaml
Doc:
  name: buy
  desc: buy shares
  spec:
    Token: buy
```

Examples:

```
buy # parses "buy"
sell # fail
```

## Space

Parses one or more whitespace characters. Fails if it isn't able to parse any whitespace characters. It has no fields.

```yaml
Space
```

## Chain

Parses an array of child parses, one after the other. Will only succeed if all child parses succeed

```yaml
Chain:
  - Token: one
  - Space
  - Token: two
  - Space
  - Token: three
```

Examples:

```
one two three # parses "one two three"
one two # fail
one twothree # fail
```

## Opt

Allows the child parser to fail, which results in parsing an empty string. When the child parser does not match, nothing is consumed and parsing continues from the same position.

```yaml
Opt:
  Token: blah
```

Examples:

```
blah # parses "blah"
something else # parses "" (nothing consumed, "something else" remains)
```

## Enum

Parses one out of a set of value strings. `exact` field determines if it requires a full match or if partial unique matches are accepted. For your own commands you should just output the whole value even if `exact: false` just to avoid any accidental ambiguity.

```yaml
Enum:
  values:
    - one
    - two
    - three
  exact: false
```

Examples:

```
one # parses "one"
o # parses "one"
two # parses "two"
t # fail
```

## Int

Parses an integer between an optional inclusive min and max. If min or max is not specified it becomes unbounded in that direction.

```yaml
Int:
  min: 3
  max: 5
```

Examples:

```
3 # parses "3"
5 # parses "5"
2 # fail
6 # fail
```

## Player

Parses a player name from the list of players playing the game. Works like an `Enum` parser with `exact: false`, so partial matches parse correctly as long as it is unique.

```yaml
Player
```

## Many

Parses a child parser many times, optionally taking a `delim` parser, as well as a `min` and `max` number of times it can match.

The `delim` parser is generally used and is generally set to `Space`.

```yaml
Many:
  spec:
    Enum:
      values:
        - one
        - two
        - three
      exact: false
  min: 1
  max: 3
  delim: Space
```

Examples

```
one # parses "one"
one two # parses "one" "two"
one one two two # parses "one" "one" "two"
onetwo # parses "one"
four # fail
```

## Real example command parser rules

This is a real set of rules that came from a game of Acquire:

```yaml
OneOf:
  - Chain:
      - Doc:
          name: buy
          desc: buy shares
          spec:
            Token: buy
      - Chain:
          - Space
          - Doc:
              name: #
              desc: number of shares to buy
              spec:
                Int:
                  min: 1
                  max: 3
      - Chain:
          - Space
          - Doc:
              name: corp
              desc: the corporation to buy shares in
              spec:
                Enum:
                  values:
                    - Worldwide
                    - Sackson
                    - Festival
                    - Imperial
                    - American
                    - Continental
                    - Tower
                  exact: false
  - Doc:
      name: done
      desc: finish buying shares and end your turn
      spec:
        Token: done
```

Example commands that are valid for these rules are listed below.

```
buy 1 worldwide
```

```
buy 2 sackson
```

```
buy 3 festival
```

```
done
```

Note that even if you provide a valid command for the command parser rules, it may still be invalid based on the game state. For example, you don't have enough money to buy shares, or there might not be enough shares left. In this case, we will provide information on the reason why your last command failed at the end of the prompt.
