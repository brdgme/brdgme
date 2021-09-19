FROM node:14.7.0 AS websocket
EXPOSE 80
WORKDIR /src
COPY websocket/package.json websocket/package-lock.json ./
RUN npm install
COPY websocket .
RUN node_modules/.bin/tsc
CMD ["node", "dist/index.js"]
