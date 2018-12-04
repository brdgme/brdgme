import * as classNames from "classnames";
import * as Immutable from "immutable";
import * as React from "react";
import * as ReactRedux from "react-redux";
import * as Redux from "redux";

import * as Records from "../records";
import { State as AppState } from "../reducers";
import * as Game from "../reducers/game";
import * as Layout from "../reducers/layout";
import * as Session from "../reducers/session";
import Player from "./player";
import { Spinner } from "./spinner";

export interface IPropValues {
  user?: Records.User;
  activeGames?: Immutable.List<Records.GameExtended>;
  menuOpen: boolean;
  onSubMenuButtonClick?: () => void;
}

interface IPropHandlers {
  onLogout: () => void;
  onRedirect: (path: string) => void;
  onToggleMenu: () => void;
  onCloseMenu: () => void;
}

interface IProps extends IPropValues, IPropHandlers { }

export class Component extends React.PureComponent<IProps, {}> {
  public constructor(props: IProps, context?: any) {
    super(props, context);

    this.handleToggleMenu = this.handleToggleMenu.bind(this);
  }

  public myNextGame(): Records.GameExtended | undefined {
    return this.props.activeGames && this.props.activeGames.find((g) => {
      return g.game_player && g.game_player.is_turn || false;
    }) || undefined;
  }

  public render() {
    let title = "brdg.me";
    const myTurnGames = this.myTurnGames().size;
    if (myTurnGames > 0) {
      title += ` (${myTurnGames})`;
    }
    const myNextGame = this.myNextGame();
    document.title = title;

    return (
      <div className="layout">
        <div className={classNames({
          "layout-header": true,
          "my-turn": myNextGame !== undefined,
        })}>
          <input type="button" onClick={this.handleToggleMenu} value="Menu" />
          <span className="header-title">brdg.me</span>
          {this.props.onSubMenuButtonClick && <input
            type="button"
            onClick={this.props.onSubMenuButtonClick}
            value="Sub menu"
          />}
          {myNextGame && <input
            type="button"
            onClick={() => this.props.onRedirect(`/game/${myNextGame.game.id}`)}
            value="Next game"
          />}
        </div>
        <div className="layout-body">
          <div className={classNames({
            menu: true,
            open: this.props.menuOpen,
          })}>
            <h1>
              <a onClick={(e) => {
                e.preventDefault();
                this.props.onRedirect("/");
                this.props.onCloseMenu();
              }}>brdg.me</a>
            </h1>
            <div className="subheading">
              <a onClick={(e) => {
                e.preventDefault();
                this.props.onRedirect("/");
                this.props.onCloseMenu();
              }}>Lo-fi board games</a>
            </div>
            {this.renderAuth()}
            <div>
              <a onClick={(e) => {
                e.preventDefault();
                this.props.onRedirect("/game/new");
                this.props.onCloseMenu();
              }}>New game</a>
            </div>
            {this.renderActiveGames()}
          </div>
          <div className="content">{this.props.children}</div>
          {this.props.menuOpen && <div
            className="menu-close-underlay"
            onClick={this.handleToggleMenu}
          />}
        </div>
      </div>
    );
  }

  private renderGame(game: Records.GameExtended): JSX.Element {
    const myPlayerId = game.game_player && game.game_player.id;
    return <div className={classNames({
      "layout-game": true,
      "my-turn": game.game_player && game.game_player.is_turn,
      "finished": game.game_player && game.game.is_finished && !game.game_player.is_read,
    })}>
      <a onClick={(e) => {
        e.preventDefault();
        this.props.onRedirect(`/game/${game.game.id}`);
        this.props.onCloseMenu();
      }}>
        <div className="layout-game-name">
          {game.game_type.name}
        </div>
        <div className="layout-game-opponents">
          with {game.game_players
            .filter((gp) => gp && gp.game_player.id !== myPlayerId || false)
            .map((gp) => <span> <Player
              name={gp.user.name}
              color={gp.game_player.color}
            /></span>)}
        </div>
      </a>
    </div >;
  }

  private renderActiveGames(): JSX.Element | undefined {
    const activeGames = this.activeGames();
    if (activeGames.size === 0) {
      return undefined;
    }
    return <div>
      <h2>Active games</h2>
      {activeGames.map((g) => g && this.renderGame(g))}
    </div>;
  }

  private activeGames(): Immutable.List<Records.GameExtended> {
    if (this.props.activeGames === undefined || this.props.user === undefined) {
      return Immutable.List();
    }
    return this.props.activeGames
      .filter((ag) => ag.game_player && (
        !ag.game.is_finished || !ag.game_player.is_read
      ))
      .sort((ag1, ag2) => {
        // Finished games at the top.
        if (ag1.game.is_finished && !ag2.game.is_finished) {
          return -1;
        }
        if (ag2.game.is_finished && !ag1.game.is_finished) {
          return 1;
        }
        // Followed by current turn games, ordered by turn time.
        if (ag1.game_player!.is_turn && ag2.game_player!.is_turn) {
          if (ag1.game_player!.is_turn_at < ag2.game_player!.is_turn_at) {
            return -1;
          }
          if (ag2.game_player!.is_turn_at < ag1.game_player!.is_turn_at) {
            return 1;
          }
        }
        if (ag1.game_player!.is_turn) {
          return -1;
        }
        if (ag2.game_player!.is_turn) {
          return 1;
        }
        // Otherwise ordered by newest first.
        if (ag1.game.updated_at > ag2.game.updated_at) {
          return -1;
        }
        return 1;
      })
      .toList();
  }

  private myTurnGames(): Immutable.List<Records.GameExtended> {
    if (this.props.activeGames === undefined || this.props.user === undefined) {
      return Immutable.List();
    }
    return this.props.activeGames
      .filter((ag) => ag.game_player && ag.game_player.is_turn || false)
      .sortBy((ag) => ag.game_player!.is_turn_at)
      .toList();
  }

  private renderAuth(): JSX.Element {
    if (this.props.user !== undefined) {
      return <div>
        <div>
          <a onClick={(e) => {
            e.preventDefault();
            this.props.onLogout();
          }}>Logout</a>
        </div>
      </div>;
    } else {
      return <div>
        <a onClick={(e) => {
          e.preventDefault();
          this.props.onRedirect("/login");
        }}>Log in</a>
      </div>;
    }
  }

  private handleToggleMenu() {
    this.props.onToggleMenu();
  }
}

interface IOwnProps {
  onSubMenuButtonClick?: () => void;
}

function mapStateToProps(state: AppState, ownProps: IOwnProps): IPropValues {
  return {
    user: state.session.user,
    activeGames: state.game.games.size > 0 && state.game.games.toList() || undefined,
    menuOpen: state.layout.menuOpen,
    onSubMenuButtonClick: ownProps.onSubMenuButtonClick,
  };
}

function mapDispatchToProps(dispatch: Redux.Dispatch<{}>): IPropHandlers {
  return {
    onLogout: () => dispatch(Session.clearToken()),
    onRedirect: (path) => dispatch(Session.updatePath(path)),
    onToggleMenu: () => dispatch(Layout.toggleMenu()),
    onCloseMenu: () => dispatch(Layout.closeMenu()),
  };
}

export const Container: React.ComponentClass<IOwnProps> = ReactRedux.connect(
  mapStateToProps,
  mapDispatchToProps,
)(Component);
