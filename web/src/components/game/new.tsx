import * as Immutable from "immutable";
import * as React from "react";
import * as ReactRedux from "react-redux";
import * as Redux from "redux";
import * as superagent from "superagent";

import * as Records from "../../records";
import { State as AppState } from "../../reducers";
import * as GameNew from "../../reducers/pages/game-new";
import { Container as Layout } from "../layout";

export interface IPropValues {
  gameVersionTypes: Immutable.List<Records.GameVersionType>;
  gameVersionId?: string;
  userIds: Immutable.List<string>;
  emails: Immutable.List<string>;
  isSubmitting: boolean;
}

export interface IPropHandlers {
  onUpdateGameVersionId: (gameVersionId: string) => void;
  onAddUserId: () => void;
  onUpdateUserId: (index: number, userId: string) => void;
  onRemoveUserId: (index: number) => void;
  onAddEmail: () => void;
  onUpdateEmail: (index: number, email: string) => void;
  onRemoveEmail: (index: number) => void;
  onSubmit: (gameVersionId: string, userIds: Immutable.List<string>, emails: Immutable.List<string>) => void;
}

export interface IProps extends IPropValues, IPropHandlers { }

export class Component extends React.PureComponent<IProps, {}> {
  constructor(props: IProps, context?: any) {
    super(props, context);

    this.handleSubmit = this.handleSubmit.bind(this);
    this.handleGameVersionSelectChange = this.handleGameVersionSelectChange.bind(this);
    this.handleAddUserIdClick = this.handleAddUserIdClick.bind(this);
    this.handleAddEmailClick = this.handleAddEmailClick.bind(this);
  }

  public render() {
    return (
      <Layout>
        <h1>New game</h1>
        <form onSubmit={this.handleSubmit}>
          <h2>Game</h2>
          <div>
            <select
              value={this.props.gameVersionId}
              onChange={this.handleGameVersionSelectChange}
            >
              <option>Choose game</option>
              {this.props.gameVersionTypes.map((gv) => <option
                value={gv.game_version.id}
              >
                {gv.game_type.name}
              </option>)}
            </select>
          </div>
          <div style={{
            display: "none",
          }}>
            <h2>Opponent IDs</h2>
            {this.props.userIds.map((uId, key) => <div>
              <input
                value={uId}
                onChange={(e) => this.handleUserIdChange(e, key)}
              />
              <a onClick={(e) => this.handleRemoveUserId(e, key)}>X</a>
            </div>)}
          </div>
          <div>
            <a onClick={this.handleAddUserIdClick}>Add</a>
          </div>
          <h2>Opponent emails</h2>
          {this.props.emails.map((email, key) => <div>
            <input
              value={email}
              onChange={(e) => this.handleEmailChange(e, key)}
            />
            <a onClick={(e) => this.handleRemoveEmail(e, key)}>X</a>
          </div>)}
          <div>
            <a onClick={this.handleAddEmailClick}>Add</a>
          </div>
          <div>
            <input type="submit" value="Create game" />
          </div>
        </form>
      </Layout>
    );
  }

  private handleSubmit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (this.props.gameVersionId === undefined) {
      return;
    }
    this.props.onSubmit(this.props.gameVersionId, this.props.userIds, this.props.emails);
  }

  private handleGameVersionSelectChange(e: React.FormEvent<HTMLSelectElement>) {
    this.props.onUpdateGameVersionId(e.currentTarget.value);
  }

  private handleAddUserIdClick(e: React.SyntheticEvent<HTMLAnchorElement>) {
    e.preventDefault();
    this.props.onAddUserId();
  }

  private handleRemoveUserId(e: React.SyntheticEvent<HTMLAnchorElement>, key: number) {
    e.preventDefault();
    this.props.onRemoveUserId(key);
  }

  private handleUserIdChange(e: React.SyntheticEvent<HTMLInputElement>, key: number) {
    this.props.onUpdateUserId(key, e.currentTarget.value);
  }

  private handleAddEmailClick(e: React.SyntheticEvent<HTMLAnchorElement>) {
    e.preventDefault();
    this.props.onAddEmail();
  }

  private handleRemoveEmail(e: React.SyntheticEvent<HTMLAnchorElement>, key: number) {
    e.preventDefault();
    this.props.onRemoveEmail(key);
  }

  private handleEmailChange(e: React.SyntheticEvent<HTMLInputElement>, key: number) {
    e.preventDefault();
    this.props.onUpdateEmail(key, e.currentTarget.value);
  }
}

function mapStateToProps(state: AppState, ownProps: {}): IPropValues {
  return {
    gameVersionTypes: state.session.gameVersionTypes || Immutable.List<Records.GameVersionType>(),
    gameVersionId: state.pages.gameNew.game_version_id,
    userIds: state.pages.gameNew.user_ids,
    emails: state.pages.gameNew.emails,
    isSubmitting: state.pages.gameNew.mode === GameNew.Mode.Submitting,
  };
}

function mapDispatchToProps(dispatch: Redux.Dispatch<GameNew.Action>, ownProps: {}): IPropHandlers {
  return {
    onUpdateGameVersionId: (gameVersionId: string) => dispatch(GameNew.updateGameVersionId(gameVersionId)),
    onAddUserId: () => dispatch(GameNew.addUserId()),
    onUpdateUserId: (index: number, userId: string) => dispatch(GameNew.updateUserId(index, userId)),
    onRemoveUserId: (index: number) => dispatch(GameNew.removeUserId(index)),
    onAddEmail: () => dispatch(GameNew.addEmail()),
    onUpdateEmail: (index: number, email: string) => dispatch(GameNew.updateEmail(index, email)),
    onRemoveEmail: (index: number) => dispatch(GameNew.removeEmail(index)),
    onSubmit: (gameVersionId: string, userIds: Immutable.List<string>, emails: Immutable.List<string>) =>
      dispatch(GameNew.submit(gameVersionId, userIds, emails)),
  };
}

export const Container: React.ComponentClass<{}> = ReactRedux.connect(
  mapStateToProps,
  mapDispatchToProps,
)(Component);
