import * as classNames from "classnames";
import * as Immutable from "immutable";
import * as moment from "moment";
import * as React from "react";
import * as ReactRedux from "react-redux";
import * as Redux from "redux";
import * as superagent from "superagent";

import * as Command from "../../command";
import * as Records from "../../records";
import { State as AppState } from "../../reducers";
import * as Game from "../../reducers/game";
import * as GameShow from "../../reducers/pages/game-show";
import * as Session from "../../reducers/session";
import * as WS from "../../reducers/ws";
import { Container as Layout } from "../layout";
import Player from "../player";
import { Spinner } from "../spinner";

const timeFormat = "h:mm A";
const dowFormat = "ddd";
const recentDateFormat = "MMM D";
const oldDateFormat = "YYYY-M-D";

export interface IPropValues extends IOwnProps {
  game?: Records.GameExtended;
  command: string;
  commandFocused: boolean;
  commandPos: number;
  submittingCommand?: boolean;
  commandError?: string;
  subMenuOpen: boolean;
}

interface IPropHandlers {
  onCommandChange: (
    command: string,
    commandPos: number,
    commandSpec?: Immutable.Map<any, any>,
  ) => void;
  onCommand: (gameId: string, command: string) => void;
  onCommandFocus: () => void;
  onCommandBlur: () => void;
  onUndo: (gameId: string) => void;
  onFetch: (gameId: string) => void;
  onRestart: (gameId: string) => void;
  onSubscribeUpdates: (gameId: string) => void;
  onUnsubscribeUpdates: (gameId: string) => void;
  onMarkRead: (gameId: string) => void;
  onConcede: (gameId: string) => void;
  onToggleSubMenu: () => void;
  onRedirect: (path: string) => void;
}

interface IProps extends IPropValues, IPropHandlers { }

export class Component extends React.PureComponent<IProps, {}> {
  constructor(props: IProps, context?: any) {
    super(props, context);

    this.onCommandInputChange = this.onCommandInputChange.bind(this);
    this.onCommandPositionChange = this.onCommandPositionChange.bind(this);
    this.focusCommandInput = this.focusCommandInput.bind(this);
    this.onCommandSubmit = this.onCommandSubmit.bind(this);
    this.handleSubMenuButtonClick = this.handleSubMenuButtonClick.bind(this);
    this.handleSuggestionsContainerClick = this.handleSuggestionsContainerClick.bind(this);
    this.handleCommandFocus = this.handleCommandFocus.bind(this);
    this.handleCommandBlur = this.handleCommandBlur.bind(this);
    this.renderMetaPlayer = this.renderMetaPlayer.bind(this);
  }

  public render(): JSX.Element {
    return (
      <Layout
        onSubMenuButtonClick={this.handleSubMenuButtonClick}
      >
        <div className="game-container">
          <div className="game-main">
            <div className="game-render">
              {this.props.game && this.props.game.html
                && <pre
                  dangerouslySetInnerHTML={{ __html: this.props.game.html }}
                />
                || <Spinner />
              }
            </div>
            {this.renderRecentLogs()}
            {this.renderSuggestionBox()}
            {!this.isMyTurn() && this.renderWhoseTurn()}
            {this.isMyTurn() && this.props.game && this.props.game.game_player && <div
              className={classNames({
                "disabled": this.props.submittingCommand,
                "game-command-input": true,
              })}
            >
              {this.props.commandError && <div className="command-error">
                {this.props.commandError}
              </div>}
              <form onSubmit={this.onCommandSubmit}>
                <input
                  ref="command"
                  type="text"
                  value={this.props.command}
                  onChange={this.onCommandInputChange}
                  onClick={this.onCommandPositionChange}
                  onKeyUp={this.onCommandPositionChange}
                  onFocus={this.handleCommandFocus}
                  onBlur={this.handleCommandBlur}
                  placeholder={!this.commandInputDisabled() && "Enter command..." || undefined}
                  disabled={this.commandInputDisabled()}
                  autoComplete="off"
                  autoCorrect="off"
                  autoCapitalize="off"
                  spellCheck={false}
                />
                <input
                  type="submit"
                  value="Send"
                  disabled={this.commandInputDisabled()}
                />
              </form>
            </div>}
          </div>
          {this.renderMeta()}
        </div>
        {this.props.subMenuOpen && <div
          className="game-meta-close-underlay"
          onClick={this.handleSubMenuButtonClick}
        />}
      </Layout>
    );
  }

  public componentDidMount() {
    this.fetchGameIfRequired(this.props);
    this.scrollToLastLog();
    this.props.onMarkRead(this.props.gameId);
    document.addEventListener("keydown", this.focusCommandInput);
    this.props.onSubscribeUpdates(this.props.gameId);
  }

  public componentWillReceiveProps(nextProps: IProps) {
    if (this.props.gameId !== nextProps.gameId) {
      this.props.onUnsubscribeUpdates(this.props.gameId);
      this.props.onSubscribeUpdates(nextProps.gameId);
      this.props.onMarkRead(nextProps.gameId);
    }
    this.fetchGameIfRequired(nextProps);
  }

  public componentDidUpdate(prevProps: IProps, prevState: {}) {
    const prevLogLen = prevProps.game
      && prevProps.game.game_logs
      && prevProps.game.game_logs.size
      || 0;
    const nextLogLen = this.props.game
      && this.props.game.game_logs
      && this.props.game.game_logs.size
      || 0;
    if (nextLogLen > prevLogLen ||
      !prevProps.subMenuOpen && this.props.subMenuOpen) {
      // New logs, scroll to bottom.
      this.scrollToLastLog();
    }
  }

  public componentWillUnmount() {
    document.removeEventListener("keydown", this.focusCommandInput);
    this.props.onUnsubscribeUpdates(this.props.gameId);
  }

  private commandSuggestions(): Command.Suggestion[] {
    if (this.props.game === undefined || !this.props.game.command_spec) {
      return [];
    }
    const commandSpec = this.props.game.command_spec.toJS();
    const fullCommand = Command.parse(this.props.command, 0, commandSpec);
    const suggestions = Command.suggestions(fullCommand, this.props.commandPos);
    let allSuggestions: Command.Suggestion[] = [];
    let start = Command.startOfMatch(fullCommand, this.props.commandPos);
    if (start === undefined) {
      // Use the end of the last match, or the start of the current word if
      // the last match ends at the end of the last word.
      const lastMatch = Command.lastMatch(fullCommand);
      if (!this.props.command.substr(lastMatch.offset, this.props.commandPos - lastMatch.offset).match(/\s/)) {
        start = lastMatch.offset;
      }
    }
    if (start !== undefined) {
      const upToStart = Command.parse(
        this.props.command.substr(0, start), 0, commandSpec);
      allSuggestions = Command.suggestions(upToStart, start);
    }
    return suggestions;
  }

  private renderRecentLogs(): JSX.Element | undefined {
    const logs = this.recentLogs();
    if (logs.size === 0) {
      return undefined;
    }
    return <div className="recent-logs-container">
      <div className="recent-logs-header">Recent logs</div>
      <div ref="recentLogs" className="recent-logs">
        {logs.map((l) => <div dangerouslySetInnerHTML={{ __html: l.html }} />)}
      </div>
    </div >;
  }

  private renderSuggestionBox(): JSX.Element | undefined {
    const suggestions = this.commandSuggestions();
    if (suggestions.length === 0) {
      return undefined;
    }
    return <div
      className="suggestions-container"
      onClick={this.handleSuggestionsContainerClick}
    >
      <div className="suggestions-content">
        {this.renderSuggestions(suggestions)}
      </div>
    </div>;
  }

  private renderSuggestionDoc(s: Command.ISuggestionDoc): JSX.Element {
    // Render doc for a single command on one line.
    if (s.values.length === 1 && s.values[0].kind === Command.SUGGESTION_VALUE) {
      return <div className="suggestion-doc-item">
        {this.renderSuggestionValueLink(s.values[0] as Command.ISuggestionValue)}
        {s.desc && <span className="suggestion-doc-desc"> - {s.desc}</span>}
      </div>;
    }
    return <div className="suggestion-doc">
      {s.desc && <div className="suggestion-doc-header">
        {s.desc && <span className="suggestion-doc-desc">{s.desc}</span>}
      </div>}
      <div className="suggestion-doc-values">
        {this.renderSuggestions(s.values)}
      </div>
    </div>;
  }

  private renderSuggestions(suggestions: Command.Suggestion[]): JSX.Element {
    // If the command input isn't focused, show a one-liner suggestion summary.
    /*
    if (!this.props.commandFocused && this.props.command === "") {
      return this.renderSuggestionsSummary(suggestions);
    }
    */
    // Render suggestions on one line if they're all values.
    if (suggestions.find((s) => s.kind === Command.SUGGESTION_DOC) === undefined) {
      const sLen = suggestions.length;
      return <div>
        {suggestions.map((s, index) => <span>
          {this.renderSuggestionValueLink(s as Command.ISuggestionValue)}
          {index < sLen - 1 && " "}
        </span>)}
      </div>;
    }
    // Otherwise render suggestions on separate lines.
    return <div>
      {suggestions.map((s) => {
        switch (s.kind) {
          case Command.SUGGESTION_VALUE:
            return <div>
              {this.renderSuggestionValueLink(s)}
            </div>;
          case Command.SUGGESTION_DOC:
            return this.renderSuggestionDoc(s);
        }
      })}
    </div>;
  }

  private recentLogs(): Immutable.List<Records.GameLogRendered> {
    if (!this.props.game || !this.props.game.game_logs || !this.props.game.game_player) {
      return Immutable.List();
    }
    const gp = this.props.game.game_player;
    return this.props.game.game_logs.filter(
      (gl) => gl.game_log.created_at >= gp.last_turn_at,
    );
  }

  private renderSuggestionsSummary(suggestions: Command.Suggestion[]): JSX.Element {
    const values = Command.suggestionValues(suggestions);
    const vLen = values.length;
    if (vLen === 0) {
      return <div />;
    }
    return <div>You can {values.map((v, index) => <span>
      <strong>{v}</strong>
      {index < vLen - 2 && ", "}
      {index === vLen - 2 && " or "}
    </span>)}</div>;
  }

  private renderSuggestionValueLink(s: Command.ISuggestionValue): JSX.Element {
    return <a onClick={(e) => {
      e.preventDefault();
      this.onCommandChange(
        this.props.command.substr(0, s.offset)
        + s.value
        + " "
        + this.props.command.substr(s.offset + (s.length || 0)),
        s.offset + s.value.length + 1,
      );
    }}>
      {s.value}
    </a>;
  }

  private commandInputDisabled(): boolean {
    if (this.props.submittingCommand) {
      return true;
    }
    if (this.props.game === undefined) {
      return true;
    }
    if (this.props.game.game.is_finished) {
      return true;
    }
    if (this.props.game.game_player === undefined
      || !this.props.game.game_player.is_turn) {
      return true;
    }
    return false;
  }

  private scrollToLastLog() {
    if (this.refs.gameLogs !== undefined) {
      const gameLogs = this.refs.gameLogs as Element;
      gameLogs.scrollTop = gameLogs.scrollHeight;
    }
    if (this.refs.recentLogs !== undefined) {
      const recentLogs = this.refs.recentLogs as Element;
      recentLogs.scrollTop = recentLogs.scrollHeight;
    }
  }

  private renderMeta(): JSX.Element {
    return <div className={classNames({
      "game-meta": true,
      "open": this.props.subMenuOpen,
    })}>
      <div className="game-meta-main">
        {this.props.game && <div>
          <h2>{this.props.game.game_type && this.props.game.game_type.name}</h2>
          {this.props.game.game_players && this.props.game.game_players.map(this.renderMetaPlayer)}
          {this.props.game.game_player && <div>
            <h3>Actions</h3>
            {!this.props.game.game.is_finished && this.props.game.game_players.size <= 2 && <div>
              <a onClick={(e) => {
                e.preventDefault();
                if (confirm("Are you sure you want to concede?")) {
                  this.props.onConcede(this.props.gameId);
                }
              }}>Concede</a>
            </div>}
            {this.props.game.game_player.can_undo && <div>
              <a onClick={(e) => {
                e.preventDefault();
                this.props.onUndo(this.props.gameId);
              }}>Undo</a>
            </div>}
            {this.props.game.game.restarted_game_id && <div>
              <a onClick={(e) => {
                e.preventDefault();
                this.props.onRedirect(`/game/${this.props.game!.game.restarted_game_id}`);
              }}>Go to restarted game</a>
            </div>}
            {this.props.game.game_player
              && this.props.game.game.is_finished
              && !this.props.game.game.restarted_game_id
              && <div>
                <a onClick={(e) => {
                  e.preventDefault();
                  this.props.onRestart(this.props.gameId);
                }}>Restart</a>
              </div>}
          </div>}
        </div>}
      </div>
      <div className="game-meta-logs">
        <h2>Logs</h2>
        <div className="game-meta-logs-content" ref="gameLogs">
          {this.renderLogs()}
        </div>
      </div>
    </div>;
  }

  private renderMetaPlayer(gp: Records.GamePlayerTypeUser): JSX.Element {
    return <div>
      <div>
        <Player
          name={gp.user.name}
          color={gp.game_player.color}
        />
      </div>
      <div style={{
        marginLeft: "1em",
      }}>
        <div>
          <abbr
            title="ELO rating, new players start at 1200"
            style={{
              cursor: "help",
            }}
          >Rating</abbr>
          :&nbsp;
          {gp.game_type_user.rating}
          {typeof gp.game_player.rating_change === "number" &&
            <span> ({this.renderRatingChange(gp.game_player.rating_change)})</span>}
        </div>
        {gp.game_player.points !== undefined && <div>
          Points: {gp.game_player.points}
        </div>}
      </div>
    </div>;
  }

  private renderRatingChange(amount: number): JSX.Element {
    return <span className="rating-change">
      {this.renderRatingChangeIcon(amount)}
      {Math.abs(amount)}
    </span>;
  }

  private renderRatingChangeIcon(amount: number): JSX.Element {
    if (amount > 0) {
      return <span className="rating-change-up">↗</span>;
    } else if (amount < 0) {
      return <span className="rating-change-down">↘</span>;
    } else {
      return <span className="rating-change-none">-</span>;
    }
  }

  private isMyTurn(): boolean {
    return this.props.game
      && this.props.game.game_player
      && this.props.game.game_player.is_turn
      || false;
  }

  private renderWhoseTurn(): JSX.Element[] {
    if (this.props.game === undefined) {
      return [];
    }
    const isMyTurn = this.isMyTurn();
    const opponentWhoseTurn = this.opponentWhoseTurn();
    const content: JSX.Element[] = [];
    if (isMyTurn) {
      content.push(<strong>Your turn!</strong>);
    }
    if (opponentWhoseTurn.size > 0) {
      const opponents = opponentWhoseTurn.map((o) => <span> <Player
        name={o.user.name}
        color={o.game_player.color}
      /></span>);
      if (isMyTurn) {
        content.push(<span> (also{opponents})</span>);
      } else {
        content.push(<span>Waiting on{opponents}</span>);
      }
    }
    return [<div className={classNames({
      "game-current-turn": true,
      "my-turn": isMyTurn,
    })}>
      {content}
    </div>];
  }

  private opponentWhoseTurn(): Immutable.List<Records.GamePlayerTypeUser> {
    return (this.props.game
      && this.props.game.game_players.filter((gp) => {
        if (gp === undefined) {
          return false;
        }
        if (gp.game_player.is_turn === false) {
          return false;
        }
        if (this.props.game!.game_player
          && this.props.game!.game_player!.id === gp.game_player.id) {
          return false;
        }
        return true;
      })
      || Immutable.List());
  }

  private fetchGameIfRequired(props: IProps) {
    if (props.game === undefined || props.game.html === undefined) {
      props.onFetch(props.gameId);
    }
  }

  private onCommandSubmit(e: React.SyntheticEvent<HTMLFormElement>) {
    e.preventDefault();
    this.props.onCommand(this.props.gameId, this.props.command.trim());
  }

  private focusCommandInput() {
    if (this.refs.command === undefined) {
      return;
    }
    (this.refs.command as HTMLInputElement).focus();
  }

  private formatLogTime(t: moment.Moment): string {
    let prefix = "";
    if (t.isBefore(moment().startOf("month").subtract(11, "months"))) {
      prefix = `${oldDateFormat}, `;
    } else if (t.isBefore(moment().startOf("day").subtract(6, "days"))) {
      prefix = `${recentDateFormat}, `;
    } else if (t.isBefore(moment().startOf("day"))) {
      prefix = `${dowFormat}, `;
    }
    return t.local().format(`${prefix}${timeFormat}`);
  }

  private renderLogs(): JSX.Element {
    if (this.props.game === undefined || this.props.game.game_logs === undefined) {
      return <div />;
    }
    const logs = this.props.game.game_logs;
    let lastLog: moment.Moment;
    const renderedLogs: JSX.Element[] = logs.map((gl) => {
      let timeEl: JSX.Element = <div />;
      const logTime = moment.utc(gl.game_log.logged_at);
      if (lastLog === undefined || logTime.clone().subtract(10, "minutes").isAfter(lastLog)) {
        timeEl = (
          <div className="log-time">- {this.formatLogTime(logTime)} -</div>
        );
      }
      lastLog = logTime;
      return (
        <div className="game-log-entry">
          {timeEl}
          <div dangerouslySetInnerHTML={{ __html: gl.html }} />
        </div>
      );
    }).toArray();
    return <div>{renderedLogs}</div>;
  }

  private commandSpec(): Immutable.Map<any, any> | undefined {
    return this.props.game
      && this.props.game.command_spec
      || undefined;
  }

  private onCommandInputChange(e: React.ChangeEvent<HTMLInputElement>) {
    this.onCommandChange(
      e.currentTarget.value,
      e.currentTarget.selectionStart,
    );
  }

  private handleCommandFocus(e: React.FormEvent<HTMLInputElement>) {
    this.props.onCommandFocus();
    this.onCommandPositionChange(e);
  }

  private onCommandPositionChange(e: React.FormEvent<HTMLInputElement>) {
    this.onCommandChange(
      this.props.command,
      e.currentTarget.selectionStart,
    );
  }

  private onCommandChange(command: string, commandPos: number) {
    if (command !== this.props.command
      || commandPos !== this.props.commandPos) {
      this.props.onCommandChange(command, commandPos, this.commandSpec());
    }
  }

  private handleSubMenuButtonClick() {
    this.props.onToggleSubMenu();
  }

  private handleSuggestionsContainerClick() {
    // this.focusCommandInput();
  }

  private handleCommandBlur() {
    // We want this to happen after click.
    setTimeout(() => {
      // Sometimes it is refocused though, so check to be sure.
      if (this.refs.command !== document.activeElement) {
        this.props.onCommandBlur();
      }
    }, 100);
  }
}

interface ICommandChange {
  command: string;
  commandPos: number;
  commandSpec: Immutable.Map<any, any>;
}

interface IOwnProps {
  gameId: string;
}

function mapStateToProps(state: AppState, ownProps: IOwnProps): IPropValues {
  return {
    gameId: ownProps.gameId,
    game: state.game.games.get(ownProps.gameId),
    command: state.pages.gameShow.command,
    commandFocused: state.pages.gameShow.commandFocused,
    commandPos: state.pages.gameShow.commandPos,
    submittingCommand: state.pages.gameShow.submittingCommand,
    commandError: state.pages.gameShow.commandError,
    subMenuOpen: state.pages.gameShow.subMenuOpen,
  };
}

function mapDispatchToProps(dispatch: Redux.Dispatch<{}>, ownProps: IOwnProps): IPropHandlers {
  return {
    onCommand: (gameId, command) => dispatch(Game.submitCommand(gameId, command)),
    onCommandFocus: () => dispatch(GameShow.commandFocus()),
    onCommandBlur: () => dispatch(GameShow.commandBlur()),
    onCommandChange: (command, commandPos, commandSpec) =>
      dispatch(GameShow.updateCommand(command, commandPos, commandSpec)),
    onToggleSubMenu: () => dispatch(GameShow.toggleSubMenu()),
    onUndo: (gameId) => dispatch(Game.submitUndo(gameId)),
    onRestart: (gameId) => dispatch(Game.submitRestart(gameId)),
    onFetch: (gameId) => dispatch(Game.fetchGame(gameId)),
    onSubscribeUpdates: (gameId) => dispatch(WS.subscribeGame(gameId)),
    onUnsubscribeUpdates: () => dispatch(WS.unsubscribeGame()),
    onMarkRead: (gameId) => dispatch(Game.submitMarkRead(gameId)),
    onConcede: (gameId) => dispatch(Game.submitConcede(gameId)),
    onRedirect: (path) => dispatch(Session.updatePath(path)),
  };
}

export const Container: React.ComponentClass<IOwnProps> = ReactRedux.connect(
  mapStateToProps,
  mapDispatchToProps,
)(Component);
