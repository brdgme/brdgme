import * as React from "react";
import * as superagent from "superagent";

import { Container as Layout } from "../layout";

export class GameIndex extends React.Component<{}, {}> {
  public render() {
    return (
      <Layout>
        <h1>Game index</h1>
      </Layout>
    );
  }
}
