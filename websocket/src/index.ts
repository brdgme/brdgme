import * as WebSocket from "ws";
import * as Redis from "redis";
import * as Log from "loglevel";
import * as Http from "http";

async function main() {
  /**
   * Configuration environment variables and defaults.
   */
  const PORT = parseInt(process.env.PORT || "80");
  const REDIS_URL = process.env.REDIS_URL || "redis://redis";
  const LOG_LEVEL = process.env.LOG_LEVEL as Log.LogLevelDesc;
  if (LOG_LEVEL !== undefined) {
    Log.setLevel(LOG_LEVEL);
  }

  /**
   * Connect to Redis, do this first as if we can't connect there's no point
   * continuing.
   */
  let redis = await Redis.createClient({
    url: REDIS_URL,
  }).connect();

  /**
   * `hasPrefix` checks whether the prefix `p` is present in the string `str`.
   * @param {string} p Prefix
   * @param {string} str String to check
   */
  function hasPrefix(p: string, str: string): boolean {
    return str.substr(0, p.length) === p;
  }

  /**
   * `Subscriptions` tracks all WebSocket connections against their subscription
   * names.
   */
  class Subscriptions {
    channels: {
      [channel: string]: WebSocket[];
    };

    constructor() {
      this.channels = {};
      this.handleMessage = this.handleMessage.bind(this);
    }

    /**
     * `subscribe` takes a `WebSocket` object and a channel name, and subscribes
     * to Redis if it's not already subscribed.
     * @param {WebSocket} ws
     * @param {string} channel
     */
    async subscribe(ws: WebSocket, channel: string) {
      if (this.channels[channel] === undefined) {
        this.channels[channel] = [];
        await redis.subscribe(channel, (message, channel) => {
          this.handleMessage(message, channel);
        });
      }
      this.channels[channel].push(ws);
      Log.info(`Subscribed ${channel}`);
    }

    /**
     * `unsubscribe` takes a `WebSocket` object and a channel name and
     * unsubscribes from Redis if it's the last subscription of this channel.
     * @param {WebSocket} ws
     * @param {string} channel
     */
    async unsubscribe(ws: WebSocket, channel: string) {
      if (this.channels[channel] === undefined) {
        return;
      }
      const index = this.channels[channel].indexOf(ws);
      if (index === -1) {
        return;
      }
      this.channels[channel].splice(index, 1);
      if (this.channels[channel].length === 0) {
        delete this.channels[channel];
        await redis.unsubscribe(channel);
      }
      Log.info(`Unsubscribed ${channel}`);
    }

    /**
     * `unsubscribeAll` takes a `WebSocket` object and unsubscribes from all
     * channels this websocket is subscribed to.
     * @param {WebSocket} ws
     */
    async unsubscribeAll(ws: WebSocket) {
      for (let channel in this.channels) {
        if (this.channels[channel].indexOf(ws) !== -1) {
          await this.unsubscribe(ws, channel);
        }
      }
    }

    /**
     * `unsubscribePrefix` takes a `WebSocket` object and a channel prefix and
     * unsubscribes the websocket from all channels it is subscribed to matching
     * the prefix.
     * @param {WebSocket} ws
     * @param {string} prefix
     */
    async unsubscribePrefix(ws: WebSocket, prefix: string) {
      for (let channel in this.channels) {
        if (
          hasPrefix(prefix, channel) &&
          this.channels[channel].indexOf(ws) !== -1
        ) {
          await this.unsubscribe(ws, channel);
        }
      }
    }

    /**
     * `unsubscribeUser` takes a `WebSocket` object and unsubscribes it from all
     * user channels it is subscribed to.
     * @param {WebSocket} ws
     */
    async unsubscribeUser(ws: WebSocket) {
      await this.unsubscribePrefix(ws, "user.");
    }

    /**
     * `unsubscribeGame` takes a `WebSocket` object and unsubscribes it from all
     * game channels it is subscribed to.
     * @param {WebSocket} ws
     */
    async unsubscribeGame(ws: WebSocket) {
      await this.unsubscribePrefix(ws, "game.");
    }

    /**
     * `handleMessage` is a `Redis` 'message' handler, proxying messages to
     * subscriptions.
     * @param {string} message
     * @param {string} channel
     */
    handleMessage(message: string, channel: string) {
      if (this.channels[channel] === undefined) {
        Log.debug(`Got message for channel ${channel}, but no subscribers`);
        return;
      }
      Log.debug(
        `Sending message to ${this.channels[channel].length} subscribers on '${channel}'`,
      );
      for (const ws of this.channels[channel]) {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(message);
        }
      }
    }
  }
  /**
   * Create our global subscription object and register it with Redis.
   */
  let subscriptions = new Subscriptions();

  /**
   * Listen for WebSocket connections. We use a custom connection to reply 200 by
   * default so Elastic Beanstalk can assume it's healthy.
   */
  let server = Http.createServer((req, res) => {
    const body = "";
    res.writeHead(200, {
      "Content-Length": body.length,
      "Content-Type": "text/plain",
    });
    res.end(body);
  });
  server.listen(PORT);
  const wss = new WebSocket.Server({ server });
  wss.on("connection", handleConnection);
  const interval = setInterval(
    () => wss.clients.forEach((ws) => ws.ping()),
    30000,
  );
  wss.on("close", () => clearInterval(interval));

  /**
   * Handles an inbound connection, listening for messages to control Redis
   * subscriptions.
   */
  function handleConnection(ws: WebSocket) {
    ws.on(
      "message",
      async (message) => await handleMessage(ws, message.toString()),
    );
    ws.on("close", async () => await handleClose(ws));
  }

  /**
   * Handles an inbound message, which manage Redis subscriptions for connections.
   */
  async function handleMessage(ws: WebSocket, message: string) {
    Log.trace(`Inbound message: ${message}`);
    let data = {};
    try {
      data = JSON.parse(message);
    } catch (e) {
      Log.debug(`Could not parse JSON message: ${message}`);
      return;
    }
    switch (data["type"]) {
      case "brdgme/ws/SUBSCRIBE_USER":
        if (data["payload"] === undefined) {
          Log.debug(`Message does not have payload: ${message}`);
          return;
        }
        await subscriptions.subscribe(ws, `user.${data["payload"]}`);
        return;
      case "brdgme/ws/UNSUBSCRIBE_USER":
        await subscriptions.unsubscribeUser(ws);
        return;
      case "brdgme/ws/SUBSCRIBE_GAME":
        if (data["payload"] === undefined) {
          Log.debug(`Message does not have payload: ${message}`);
          return;
        }
        await subscriptions.subscribe(ws, `game.${data["payload"]}`);
        return;
      case "brdgme/ws/UNSUBSCRIBE_GAME":
        await subscriptions.unsubscribeGame(ws);
        return;
      default:
        Log.debug(`Invalid message type: ${message}`);
        return;
    }
  }

  /**
   * Handles the close of a websocket, unsubscribing it from all subscriptions.
   */
  async function handleClose(ws: WebSocket) {
    await subscriptions.unsubscribeAll(ws);
  }
}

main();
