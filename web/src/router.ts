type Handler<T> = (input: string) => T | undefined;

export function first<T>(
  input: string,
  handlers: Array<Handler<T>>,
): T | undefined {
  for (const h of handlers) {
    const res = h(input);
    if (res !== undefined) {
      return res;
    }
  }
  return undefined;
}

export function prefix<T>(
  p: string,
  success: (remaining: string) => T | undefined,
): Handler<T> {
  return (input: string) => {
    if (input.substr(0, p.length) === p) {
      return success(input.substr(p.length));
    }
    return undefined;
  };
}

export function match<T>(
  p: string,
  success: () => T | undefined,
): Handler<T> {
  return (input: string) => {
    if (input === p) {
      return success();
    }
    return undefined;
  };
}

export function empty<T>(
  success: () => T | undefined,
): Handler<T> {
  return match("", success);
}

export function any<T>(
  success: () => T | undefined,
): Handler<T> {
  return (input: string) => success();
}

const intRegex = /^-?[0-9]+/;
export function int<T>(
  success: (n: number, remaining: string) => T | undefined,
): Handler<T> {
  return (input: string) => {
    const res = intRegex.exec(input);
    if (!res) {
      return undefined;
    }
    return success(parseInt(res[0], 10), input.substr(res[0].length));
  };
}
