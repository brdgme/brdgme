import * as React from "react";
import * as ReactRedux from "react-redux";
import * as Redux from "redux";

import { State as AppState } from "../reducers";
import * as LoginReducer from "../reducers/pages/login";
import { Spinner } from "./spinner";

export interface IPropValues {
  email: string;
  code: string;
  mode: LoginReducer.Mode;
}
export interface IPropHandlers {
  onChangeEmail: (email: string) => void;
  onChangeCode: (code: string) => void;
  onChangeMode: (mode: LoginReducer.Mode) => void;
  onSubmitEmail: (email: string) => void;
  onSubmitCode: (email: string, code: string) => void;
}
export interface IProps extends IPropValues, IPropHandlers { }

export class Component extends React.PureComponent<IProps, {}> {
  constructor(props: IProps, context?: any) {
    super(props, context);

    this.handleChangeCode = this.handleChangeCode.bind(this);
    this.handleChangeEmail = this.handleChangeEmail.bind(this);
    this.handleClickChangeEmail = this.handleClickChangeEmail.bind(this);
    this.handleClickHaveCode = this.handleClickHaveCode.bind(this);
    this.handleSubmitCode = this.handleSubmitCode.bind(this);
    this.handleSubmitEmail = this.handleSubmitEmail.bind(this);
  }

  public render() {
    return (
      <div className="login">
        <h1>brdg.me</h1>
        <div className="subtitle">
          Lo-fi board games, email / web
        </div>
        {this.props.mode === LoginReducer.Mode.EnteringEmail && (
          <div>
            <div>Enter your email address to start</div>
            <form onSubmit={this.handleSubmitEmail}>
              <div>
                <input
                  type="email"
                  required
                  autoFocus
                  placeholder="Email address"
                  value={this.props.email}
                  onChange={this.handleChangeEmail}
                />
                <input type="submit" value="Get code" />
              </div>
              <div className="hasCode">
                <a onClick={this.handleClickHaveCode}>I already have a login code</a>
              </div>
            </form>
          </div>
        ) || (
            <div>
              Logging in as
              <a onClick={this.handleClickChangeEmail}>{this.props.email}</a>
            </div>
          )}
        {this.props.mode === LoginReducer.Mode.EnteringCode && (
          <div>
            <div>A login code has been sent to your email, please enter it here</div>
            <form onSubmit={this.handleSubmitCode}>
              <input
                type="tel"
                pattern="[0-9]*"
                required
                autoFocus
                placeholder="Login code"
                value={this.props.code}
                onChange={this.handleChangeCode}
              />
              <input type="submit" value="Play!" />
            </form>
          </div>
        )}
        {(this.props.mode === LoginReducer.Mode.SubmittingEmail ||
          this.props.mode === LoginReducer.Mode.SubmittingCode) && (
            <Spinner />
          )}
      </div>
    );
  }

  private handleSubmitEmail(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    this.props.onSubmitEmail(this.props.email);
  }

  private handleSubmitCode(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    this.props.onSubmitCode(this.props.email, this.props.code);
  }

  private handleClickHaveCode(e: React.MouseEvent<HTMLAnchorElement>) {
    e.preventDefault();
    const form = (e.currentTarget.parentElement!.parentElement) as HTMLFormElement;
    if (form.reportValidity()) {
      this.props.onChangeMode(LoginReducer.Mode.EnteringCode);
    }
  }

  private handleClickChangeEmail(e: React.FormEvent<HTMLAnchorElement>) {
    e.preventDefault();
    this.props.onChangeMode(LoginReducer.Mode.EnteringEmail);
  }

  private handleChangeCode(e: React.ChangeEvent<HTMLInputElement>) {
    this.props.onChangeCode(e.target.value);
  }

  private handleChangeEmail(e: React.ChangeEvent<HTMLInputElement>) {
    this.props.onChangeEmail(e.target.value);
  }
}

function mapStateToProps(state: AppState): IPropValues {
  return {
    code: state.pages.login.code,
    email: state.pages.login.email,
    mode: state.pages.login.mode,
  };
}

function mapDispatchToProps(dispatch: Redux.Dispatch<{}>): IPropHandlers {
  return {
    onChangeCode: (code) => dispatch(LoginReducer.updateCode(code)),
    onChangeEmail: (email) => dispatch(LoginReducer.updateEmail(email)),
    onChangeMode: (mode) => dispatch(LoginReducer.updateMode(mode)),
    onSubmitCode: (email, code) => dispatch(LoginReducer.submitCode(email, code)),
    onSubmitEmail: (email) => dispatch(LoginReducer.submitEmail(email)),
  };
}

export const Container: React.ComponentClass<{}> = ReactRedux.connect(
  mapStateToProps,
  mapDispatchToProps,
)(Component);
