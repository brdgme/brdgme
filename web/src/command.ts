export interface ICommandSpec {
  Int?: { min?: number; max?: number };
  Token?: string;
  Enum?: { values: string[]; exact: boolean };
  OneOf?: CommandSpec[];
  Chain?: CommandSpec[];
  Many?: { min?: number; max?: number; delim?: CommandSpec; spec: CommandSpec };
  Opt?: CommandSpec;
  Doc?: { name: string; desc?: string; spec: CommandSpec };
}

export const COMMAND_SPEC_PLAYER = "Player";
export const COMMAND_SPEC_SPACE = "Space";

export type CommandSpec =
  | ICommandSpec
  | typeof COMMAND_SPEC_PLAYER
  | typeof COMMAND_SPEC_SPACE;

export interface IParseResult {
  kind: typeof MATCH_FULL | typeof MATCH_PARTIAL | typeof MATCH_ERROR;
  offset: number;
  length?: number;
  next?: IParseResult[];
  value?: string;
  name?: string;
  desc?: string;
  message?: string;
}

// Match kinds are ordered strings so that FULL > PARTIAL > ERROR
export const MATCH_FULL = "2_MATCH_FULL";
export const MATCH_PARTIAL = "1_MATCH_PARTIAL";
export const MATCH_ERROR = "0_MATCH_ERROR";

/**
 * Generates some potential values for the int parser to use for suggestions.
 * @param min minimum value
 * @param max maximum value
 */
function potentialIntValues(min?: number, max?: number): number[] {
  const start = min || 1;
  const end = max || start + 4;
  const values: number[] = [];
  for (let i = start; i <= end; i++) {
    values.push(i);
  }
  return values;
}

const intRegex = /\-?\d+/;
export function parseIntSpec(
  input: string,
  offset: number,
  min?: number,
  max?: number,
): IParseResult {
  // First we see if we have an actual integer match.
  const intMatches = intRegex.exec(input.substr(offset));
  // We also do an enum match so we can provide suggestions.
  const enumValues = potentialIntValues(min, max)
    .map((i) => i.toString())
    .filter((i) => !intMatches || intMatches[0] !== i);
  const enumResult = parseEnum(input, offset, enumValues, true);
  let intResult: IParseResult = {
    kind: MATCH_PARTIAL,
    offset,
  };
  if (intMatches) {
    const value = parseInt(intMatches[0], 10);
    if (min !== undefined && min !== null && value < min) {
      intResult = {
        kind: MATCH_ERROR,
        offset,
        message: `${value} is less than the minimum ${min}`,
      };
    } else if (max !== undefined && max !== null && value > max) {
      intResult = {
        kind: MATCH_ERROR,
        offset,
        message: `${value} is greater than the maximum ${max}`,
      };
    } else {
      intResult = {
        kind: MATCH_FULL,
        offset,
        length: intMatches[0].length,
        value: intMatches[0],
      };
    }
  }
  let next;
  if (intResult.kind !== MATCH_FULL) {
    next = enumResult.next;
  }
  return {
    kind: intResult.kind,
    offset,
    length: intResult.length,
    value: intResult.value,
    next,
  };
}

export function commonPrefix(s1: string, s2: string): string {
  const iterBound = Math.min(s1.length, s2.length);
  for (let i = 0; i < iterBound; i++) {
    if (s1.charAt(i) !== s2.charAt(i)) {
      return s1.substr(0, i);
    }
  }
  return s1.substr(0, iterBound);
}

export function parseEnum(
  input: string,
  offset: number,
  values: string[],
  exact: boolean,
): IParseResult {
  let matches: IParseResult[] = [];
  let length = 0;
  for (const v of values) {
    const result = parseToken(input, offset, v);
    if (result.kind !== MATCH_ERROR) {
      const matchLen = result.length || 0;
      if (matchLen > length) {
        matches = [];
        length = matchLen;
      }
      if (matchLen === length) {
        matches.push(result);
      }
    }
  }

  if (matches.length === 0) {
    return {
      kind: MATCH_ERROR,
      offset,
      message: `input doesn't match any value in: ${values.join(", ")}`,
    };
  }
  if (matches.length === 1 && (matches[0].length || 0) > 0) {
    return Object.assign({}, matches[0], {
      kind:
        ((matches[0].kind === MATCH_FULL || !exact) && MATCH_FULL) ||
        MATCH_PARTIAL,
    });
  }
  for (const m of matches) {
    if (m.kind === MATCH_FULL) {
      return m;
    }
  }
  // Because we have multiple partial matches, we return this as a zero
  // length full match with all the partial matches as children.
  return {
    kind: MATCH_FULL,
    offset,
    next: matches,
  };
}

export function parseToken(
  input: string,
  offset: number,
  token: string,
): IParseResult {
  if (offset >= input.length) {
    return {
      kind: MATCH_PARTIAL,
      offset,
      value: token,
    };
  }
  const tLen = token.length;
  if (tLen === 0) {
    return {
      kind: MATCH_FULL,
      offset,
      value: "",
    };
  }
  const prefix = commonPrefix(
    input.substr(offset, tLen).toLowerCase(),
    token.toLowerCase(),
  );
  const prefixLen = prefix.length;
  if (prefixLen === 0) {
    return {
      kind: MATCH_ERROR,
      offset,
      message: `'${token}' not found`,
    };
  }
  return {
    kind: (tLen === prefixLen && MATCH_FULL) || MATCH_PARTIAL,
    offset,
    length: prefixLen,
    value: token,
  };
}

const spaceRegex = /^\s+/;
export function parseSpace(input: string, offset: number): IParseResult {
  if (offset >= input.length) {
    return {
      kind: MATCH_PARTIAL,
      offset,
    };
  }
  const matches = spaceRegex.exec(input.substr(offset));
  if (matches) {
    return {
      kind: MATCH_FULL,
      offset,
      length: matches[0].length,
      value: input.slice(offset, offset + matches[0].length),
    };
  }
  return {
    kind: MATCH_ERROR,
    offset,
    message: "expected a space",
  };
}

export function parseOneOf(
  input: string,
  offset: number,
  specs: CommandSpec[],
  names: string[],
): IParseResult {
  let success = 0;
  const results: IParseResult[] = specs.map((s) => {
    const res = parse(input, offset, s, names);
    if (res.kind !== MATCH_ERROR) {
      success++;
    }
    return res;
  });
  return {
    kind: MATCH_FULL,
    offset,
    next: results,
  };
}

export function parseDoc(
  input: string,
  offset: number,
  name: string,
  spec: CommandSpec,
  names: string[],
  desc?: string,
): IParseResult {
  return {
    kind: MATCH_FULL,
    offset,
    name,
    desc,
    next: [parse(input, offset, spec, names)],
  };
}

export interface IFlatResult {
  flat: IParseResult;
  combined: IParseResult;
}
/**
 * Flattens a result into the best matching branch, as well as a combined
 * version of the best matching branch.
 */
export function flattenResult(result: IParseResult): IFlatResult {
  if (
    result.kind !== MATCH_FULL ||
    result.next === undefined ||
    result.next.length === 0
  ) {
    return {
      flat: result,
      combined: result,
    };
  }
  let best: IFlatResult | undefined;
  for (const n of result.next) {
    const nFlat = flattenResult(n);
    if (
      best === undefined ||
      nFlat.combined.kind > best.combined.kind ||
      (nFlat.combined.kind === best.combined.kind &&
        (nFlat.combined.length || 0) > (best.combined.length || 0))
    ) {
      best = nFlat;
    }
  }
  return {
    flat: Object.assign({}, result, {
      next: [best!.flat],
    }),
    combined: Object.assign({}, result, best, {
      kind: best!.combined.kind,
      offset: result.offset,
      length: (result.length || 0) + (best!.combined.length || 0),
      value: (result.value || "") + (best!.combined.value || ""),
    }),
  };
}

/**
 * Appends the result as leaves to every branch of the tree.
 */
export function pushResult(
  result: IParseResult,
  to: IParseResult,
): IParseResult {
  if (to.next === undefined || to.next.length === 0) {
    return Object.assign({}, to, {
      next: [result],
    });
  }
  return Object.assign({}, to, {
    next: to.next.map((n) => pushResult(result, n)),
  });
}

export const SUGGESTION_DOC = "SUGGESTION_DOC";
export const SUGGESTION_VALUE = "SUGGESTION_VALUE";

export interface ISuggestionDoc {
  kind: typeof SUGGESTION_DOC;
  offset: number;
  length?: number;
  desc?: string;
  values: Suggestion[];
}
export interface ISuggestionValue {
  kind: typeof SUGGESTION_VALUE;
  offset: number;
  length?: number;
  value: string;
}
export type Suggestion = ISuggestionDoc | ISuggestionValue;
export function suggestions(result: IParseResult, at: number): Suggestion[] {
  let s: Suggestion[] = [];
  let nextValues: Suggestion[] = [];
  if (result.next !== undefined) {
    for (const n of result.next) {
      nextValues = nextValues.concat(suggestions(n, at));
    }
  }
  if (result.kind !== MATCH_ERROR && nextValues.length === 0) {
    const offset = result.offset || 0;
    const length = result.length || 0;
    if (result.value !== undefined && offset <= at && offset + length >= at) {
      s.push({
        kind: SUGGESTION_VALUE,
        offset: result.offset,
        length: result.length,
        value: result.value,
      });
    }
  }
  s = s.concat(nextValues);
  if (s.length > 0 && result.desc !== undefined) {
    // We only document values which have a matching offset as the result, so
    // split them here.
    const docS: Suggestion[] = [];
    const valueS: Suggestion[] = [];
    for (const v of s) {
      if (v.offset === result.offset) {
        docS.push(v);
      } else {
        valueS.push(v);
      }
    }
    const splitS: Suggestion[] = [];
    if (docS.length > 0) {
      splitS.push({
        kind: SUGGESTION_DOC,
        offset: result.offset,
        length: result.length,
        desc: result.desc,
        values: docS,
      });
    }
    return splitS.concat(valueS);
  }
  return s;
}

export function suggestionValues(ss: Suggestion[]): string[] {
  let values: string[] = [];
  for (const s of ss) {
    switch (s.kind) {
      case SUGGESTION_VALUE:
        values.push(s.value);
        break;
      case SUGGESTION_DOC:
        values = values.concat(suggestionValues(s.values));
        break;
    }
  }
  return values;
}

export function startOfMatch(
  result: IParseResult,
  at: number,
): number | undefined {
  if (at === 0) {
    return 0;
  }
  if (
    result.value !== undefined &&
    (result.length || 0) > 0 &&
    result.offset <= at &&
    result.offset + (result.length || 0) >= at
  ) {
    return result.offset;
  }
  if (result.next !== undefined) {
    for (const n of result.next) {
      const ns = startOfMatch(n, at);
      if (ns !== undefined) {
        return ns;
      }
    }
  }
}

export function lastMatch(result: IParseResult): IParseResult {
  let last = result;
  let lastPos = result.offset + (result.length || 0);
  if (result.next !== undefined) {
    for (const n of result.next) {
      const nl = lastMatch(n);
      const nlPos = nl.offset + (nl.length || 0);
      if (nlPos > lastPos) {
        last = nl;
        lastPos = nlPos;
      }
    }
  }
  return last;
}

export function parseChain(
  input: string,
  offset: number,
  specs: CommandSpec[],
  names: string[],
): IParseResult {
  const headSpec = specs[0];
  const tailSpecs = specs.slice(1);
  const result = parse(input, offset, headSpec, names);
  const flatResult = flattenResult(result);
  if (flatResult.combined.kind !== MATCH_FULL || tailSpecs.length === 0) {
    // No full match on this link of the chain or end of the chain, exit here.
    return result;
  }
  const tailResult = parseChain(
    input,
    offset + (flatResult.combined.length || 0),
    tailSpecs,
    names,
  );
  return pushResult(tailResult, flatResult.flat);
}

export function parseMany(
  input: string,
  offset: number,
  spec: CommandSpec,
  names: string[],
  delim?: CommandSpec,
  min?: number,
  max?: number,
): IParseResult {
  let result: IParseResult | undefined;
  let matches = 0;
  let offsetIter = offset;

  while (true) {
    if (result && delim) {
      const delimResult = parse(input, offsetIter, delim, names);
      result = pushResult(delimResult, result);
      const delimFlatResult = flattenResult(delimResult);
      if (delimFlatResult.combined.kind !== MATCH_FULL) {
        return result;
      }
      offsetIter = delimResult.offset + (delimResult.length || 0);
    }

    const itemResult = parse(input, offsetIter, spec, names);
    const itemFlatResult = flattenResult(itemResult);
    if (itemFlatResult.combined.kind === MATCH_FULL) {
      matches++;
    }

    if (!result) {
      // If we don't have a top level result yet, we just use this one.
      result = itemResult;
    } else {
      // Otherwise we only append if relevant.
      if (
        itemFlatResult.combined.kind === MATCH_ERROR &&
        min !== undefined &&
        matches < min
      ) {
        // We didn't parse another and haven't met the minimum yet, so this is an
        // error.
        result = pushResult(itemResult, result);
        result.kind = MATCH_ERROR;
        return result;
      }

      if (itemFlatResult.combined.kind !== MATCH_ERROR) {
        result = pushResult(itemResult, result);
      }
    }

    if (
      itemFlatResult.combined.kind !== MATCH_FULL ||
      (max !== undefined && matches == max)
    ) {
      if (min !== undefined && matches < min) {
        result.kind = MATCH_PARTIAL;
      }
      return result;
    }

    offsetIter = itemResult.offset + (itemResult.length || 0);
  }
}

export function parseOpt(
  input: string,
  offset: number,
  spec: CommandSpec,
  names: string[],
): IParseResult {
  const subResult = parse(input, offset, spec, names);
  if (subResult.kind == MATCH_ERROR) {
    return {
      kind: MATCH_FULL,
      offset,
      length: 0,
      value: "",
    };
  }
  return subResult;
}

export function parsePlayer(
  input: string,
  offset: number,
  names: string[],
): IParseResult {
  return parseEnum(input, offset, names, false);
}

export function parse(
  input: string,
  offset: number,
  spec: CommandSpec,
  names: string[],
): IParseResult {
  if (spec === COMMAND_SPEC_PLAYER) {
    return parsePlayer(input, offset, names);
  } else if (spec === COMMAND_SPEC_SPACE) {
    return parseSpace(input, offset);
  } else if (spec.Int !== undefined) {
    return parseIntSpec(input, offset, spec.Int.min, spec.Int.max);
  } else if (spec.Token !== undefined) {
    return parseToken(input, offset, spec.Token);
  } else if (spec.Enum !== undefined) {
    return parseEnum(input, offset, spec.Enum.values, spec.Enum.exact);
  } else if (spec.OneOf !== undefined) {
    return parseOneOf(input, offset, spec.OneOf, names);
  } else if (spec.Chain !== undefined) {
    return parseChain(input, offset, spec.Chain, names);
  } else if (spec.Many !== undefined) {
    return parseMany(
      input,
      offset,
      spec.Many.spec,
      names,
      spec.Many.delim,
      spec.Many.min,
      spec.Many.max,
    );
  } else if (spec.Opt !== undefined) {
    return parseOpt(input, offset, spec.Opt, names);
  } else if (spec.Doc !== undefined) {
    return parseDoc(
      input,
      offset,
      spec.Doc.name,
      spec.Doc.spec,
      names,
      spec.Doc.desc,
    );
  }
  return {
    kind: MATCH_ERROR,
    offset,
    message: "invalid command spec",
  };
}
