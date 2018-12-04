import * as React from "react";
import * as ReactRedux from "react-redux";
import * as Redux from "redux";
import * as superagent from "superagent";

import { ISession } from "../model";
import { State as AppState } from "../reducers";
import { Container as Layout } from "./layout";

export class Component extends React.PureComponent<{}, {}> {
  public render() {
    return (
      <Layout
      >
        <h1>Home blah</h1>
      </Layout>
    );
  }
}

function mapStateToProps(state: AppState): {} {
  return {};
}

function mapDispatchToProps(dispatch: Redux.Dispatch<{}>): {} {
  return {};
}

export const Container = ReactRedux.connect(
  mapStateToProps,
  mapDispatchToProps,
)(Component);
