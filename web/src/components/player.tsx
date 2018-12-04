import * as React from "react";

export interface IProps {
  name: string;
  color: string;
}

const camelRegex = /([A-Z])/g;

function camelToKebab(s: string): string {
  if (s.length === 0) {
    return s;
  }
  return s.substr(0, 1).toLowerCase() +
    s.substr(1).replace(camelRegex, (_, b: string) => `-${b.toLowerCase()}`);
}

export default class Player extends React.PureComponent<IProps, {}> {
  public render() {
    return <strong className={`brdgme-${camelToKebab(this.props.color)}`}>
      &lt;{this.props.name}&gt;
    </strong>;
  }
}
