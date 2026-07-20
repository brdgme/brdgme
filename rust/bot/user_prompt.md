# Players

You are player **{{ my_name }}** and have the colour {{ my_colour }}.

All of the current players in the game are listed below, including yourself. If you ever need to specify a player in your command, you will need to use a name from this list.

{% for player in players %}
- {{ player.name }}{% if player.name == my_name %} (you){% endif %}
  - Score: {{ player.score }}
  - Colour: {{ player.colour }}
{% endfor %}

# Public game data

The public game state, in YAML format, is below.

```yaml
{{ pub_state_yaml }}
```

# Your private game data

Your private game state (information visible only to you), in YAML format, is below.

```yaml
{{ player_state_yaml }}
```

# Command parser rules for your turn right now

The command you respond with **must** be valid based on the command parser rules below. You must only respond with a single command. If your turn is not over after your command and you need to take more actions, you **must not** provide more than one command at a time but will be prompted separately for follow up commands. You must respond with only a single valid command as per the parser rules, and not include any other text, explanation or information.

The command parser rules below, provided in YAML format. These rules exactly match the parsers defined in the "Command parser rules" section earlier in the document.

```yaml
{{ command_spec }}
```

# Recent game logs

The 20 most recent game logs are listed below in chronological order to help you understand what has happened recently in the game.

Game logs are in brdgme markup, as documented above.

{% for log in recent_logs %}
- {{ log }}
{% endfor %}

{% if failed_commands %}
You have previously responded with commands that have failed, which are all listed below:

{% for failed in failed_commands %}
- Command: {{ failed.command }}
  - Error: {{ failed.error }}
{% endfor %}
{% endif %}

Respond now with a single valid command on a single line without any other additional text or explanation.

Please provide your command now.
