import { assert } from "chai";
import "mocha";

import * as Command from "../src/command";

describe("Command.parseWhitespace", () => {
  it("should parse leading whitespace", () => {
    const result = Command.parseSpace("   hello ", 0);
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 3);
    assert.equal(result.value, "   ");
  });
  it("should parse newlines", () => {
    const result = Command.parseSpace(`
  hello `, 0);
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 3);
    assert.equal(result.value, `
  `);
  });
});

describe("Command.commonPrefix", () => {
  it("should do partial matches", () => {
    assert.equal("fart", Command.commonPrefix("fartbag", "fartdog"));
  });
  it("should be case sensitive", () => {
    assert.equal("", Command.commonPrefix("Fartbag", "fartdog"));
  });
  it("should match the full first string", () => {
    assert.equal("fart", Command.commonPrefix("fart", "fartdog"));
  });
  it("should match the full second string", () => {
    assert.equal("fart", Command.commonPrefix("fartbag", "fart"));
  });
});

describe("Command.parseToken", () => {
  it("should match full token", () => {
    const result = Command.parseToken("fart   ", 0, "fart");
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 4);
    assert.equal(result.value, "fart");
  });
  it("should be case insensitive", () => {
    const result = Command.parseToken("FaRt   ", 0, "fart");
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 4);
    assert.equal(result.value, "fart");
  });
  it("should partially match", () => {
    const result = Command.parseToken("FaR   ", 0, "fart");
    assert.equal(result.kind, Command.MATCH_PARTIAL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 3);
    assert.equal(result.value, "fart");
  });
});

describe("Command.parseInt", () => {
  it("should parse positive numbers", () => {
    const result = Command.parseIntSpec("264   ", 0);
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 3);
    assert.equal(result.value, "264");
  });
  it("should parse negative numbers", () => {
    const result = Command.parseIntSpec("-264   ", 0);
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 4);
    assert.equal(result.value, "-264");
  });
  it("should parse numbers above min", () => {
    const result = Command.parseIntSpec("-264   ", 0, -300);
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 4);
    assert.equal(result.value, "-264");
  });
  it("should fail to parse numbers below min", () => {
    const result = Command.parseIntSpec("-264   ", 0, -20);
    assert.equal(result.kind, Command.MATCH_ERROR);
  });
  it("should parse numbers below max", () => {
    const result = Command.parseIntSpec("-264   ", 0, undefined, -5);
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 4);
    assert.equal(result.value, "-264");
  });
  it("should fail to parse numbers above max", () => {
    const result = Command.parseIntSpec("-264   ", 0, undefined, -300);
    assert.equal(result.kind, Command.MATCH_ERROR);
  });
});

describe("Command.parseEnum", () => {
  it("should full match exact matches", () => {
    const result = Command.parseEnum("Fartbag", 0, ["fart", "Fartb"], false);
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 5);
    assert.equal(result.value, "Fartb");
  });
  it("should partial match all equal length common prefixes", () => {
    const result = Command.parseEnum("Fargoo", 0, ["fart", "Fartb", "fae"], false);
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.isNotNull(result.next);
    assert.lengthOf(result.next!, 2);
    assert.equal(result.next![0].value, "fart");
    assert.equal(result.next![1].value, "Fartb");
  });
  it("should full match unique common prefix", () => {
    const result = Command.parseEnum("Fargoo", 0, ["fart", "Fam", "fae"], false);
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 3);
    assert.equal(result.value, "fart");
  });
});

describe("Command.parseMany", () => {
  it("should parse a single item", () => {
    const result = Command.flattenResult(Command.parseMany(
      "fart   ",
      0,
      {
        Token: "fart",
      },
      [],
    )).combined;
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 4);
    assert.equal(result.value, "fart");
  });
  it("should parse multiple items without delim", () => {
    const result = Command.parseMany(
      "fartfart",
      0,
      {
        Token: "fart",
      },
      []
    );
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 8);
    assert.equal(result.value, "fartfart");
  });
  it("should parse multiple items with delim", () => {
    const result = Command.parseMany(
      "fart   fart",
      0,
      {
        Token: "fart",
      },
      [],
      Command.COMMAND_SPEC_SPACE,
    );
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 11);
    assert.equal(result.value, "fart   fart");
  });
  it("should parse partial item", () => {
    const result = Command.parseMany(
      "fartcheesefar",
      0,
      {
        Token: "fart",
      },
      [],
      {
        Token: "cheese",
      },
    );
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 4);
    assert.equal(result.value, "fart");
    assert.isNotNull(result.next);
    assert.lengthOf(result.next!, 1);
    assert.equal(result.next![0].kind, Command.MATCH_PARTIAL);
    assert.equal(result.next![0].offset, 10);
    assert.equal(result.next![0].length, 3);
    assert.equal(result.next![0].value, "fart");
  });
  it("should parse partial delim", () => {
    const result = Command.parseMany(
      "fartche",
      0,
      {
        Token: "fart",
      },
      [],
      {
        Token: "cheese",
      },
    );
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 4);
    assert.equal(result.value, "fart");
    assert.isNotNull(result.next);
    assert.lengthOf(result.next!, 1);
    assert.equal(result.next![0].kind, Command.MATCH_PARTIAL);
    assert.equal(result.next![0].offset, 4);
    assert.equal(result.next![0].length, 3);
    assert.equal(result.next![0].value, "cheese");
  });
  it("should support nested enums", () => {
    const result = Command.parseMany(
      "fart  cheese b",
      0,
      {
        Enum: {
          values: [
            "fart",
            "cheese",
            "bacon",
            "tomato",
            "banana",
          ],
          exact: false,
        },
      },
      [],
      Command.COMMAND_SPEC_SPACE,
    );
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 12);
    assert.equal(result.value, "fart  cheese");
    assert.isNotNull(result.next);
    assert.lengthOf(result.next!, 1);
    assert.isNotNull(result.next![0].next);
    assert.lengthOf(result.next![0].next!, 2);
    assert.equal(result.next![0].next![0].kind, Command.MATCH_PARTIAL);
    assert.equal(result.next![0].next![0].offset, 13);
    assert.equal(result.next![0].next![0].length, 1);
    assert.equal(result.next![0].next![1].kind, Command.MATCH_PARTIAL);
    assert.equal(result.next![0].next![1].offset, 13);
    assert.equal(result.next![0].next![1].length, 1);
  });
  it("should parse up to the max", () => {
    const result = Command.parseMany(
      "fartfartfart",
      0,
      {
        Token: "fart",
      },
      [],
      undefined,
      undefined,
      2,
    );
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 8);
    assert.equal(result.value, "fartfart");
  });
  it("should be partial if min not met", () => {
    const result = Command.parseMany(
      "fartfartfart",
      0,
      {
        Token: "fart",
      },
      [],
      undefined,
      4,
    );
    assert.equal(result.kind, Command.MATCH_PARTIAL);
    assert.equal(result.offset, 0);
    assert.equal(result.length, 12);
    assert.equal(result.value, "fartfartfart");
  });
  it("should be partial when enum and no input yet", () => {
    const result = Command.parseMany(
      "",
      0,
      {
        Enum: {
          values: [
            "fart",
            "cheese",
            "bacon",
          ],
          exact: false,
        },
      },
      [],
      Command.COMMAND_SPEC_SPACE,
    );
    assert.equal(result.kind, Command.MATCH_FULL);
    assert.equal(result.offset, 0);
    assert.equal(result.value, "");
    assert.isNotNull(result.next);
    assert.lengthOf(result.next!, 3);
    assert.equal(result.next![0].kind, Command.MATCH_PARTIAL);
    assert.equal(result.next![0].offset, 0);
    assert.equal(result.next![1].kind, Command.MATCH_PARTIAL);
    assert.equal(result.next![1].offset, 0);
    assert.equal(result.next![2].kind, Command.MATCH_PARTIAL);
    assert.equal(result.next![2].offset, 0);
  });
  it("should recreate the value from the input", () => {
    const result = Command.flattenResult(Command.parseMany(
      "1 2 ",
      0,
      {
        Int: {
          min: 1,
          max: 2,
        }
      },
      [],
      Command.COMMAND_SPEC_SPACE,
    )).combined;
    assert.equal(result.kind, Command.MATCH_PARTIAL);
    assert.equal(result.offset, 0);
    assert.equal(result.value, "1 2");
  });
});
