import * as React from "react";
import * as ReactRedux from "react-redux";
import * as Redux from "redux";

import * as Model from "../model";
import { State as AppState } from "../reducers";
import * as Router from "../router";
import { GameIndex } from "./game/index";
import { Container as GameNew } from "./game/new";
import { Container as GameShow } from "./game/show";
import { Container as Home } from "./home";
import { Container as Login } from "./login";

interface IPropValues {
  path: string;
}
export class Component extends React.PureComponent<IPropValues, {}> {
  public render() {
    return Router.first(this.props.path, [
      Router.match("/login", () => <Login />),
      Router.prefix("/game", (remaining) =>
        Router.first(remaining, [
          Router.match("/new", () => <GameNew />),
          Router.empty(() => <GameIndex />),
          Router.any(() => <GameShow gameId={remaining.substr(1)} />),
        ]),
      ),
      Router.any(() => <Home />),
    ]) || <div />;
  }
}

function mapStateToProps(state: AppState): IPropValues {
  return {
    path: state.session.path,
  };
}

function mapDispatchToProps(dispatch: Redux.Dispatch<{}>): {} {
  return {};
}

export const Container: React.ComponentClass<{}> = ReactRedux.connect(
  mapStateToProps,
  mapDispatchToProps,
)(Component);
