FROM node:14.7.0 AS web-src
WORKDIR /src
COPY web/package.json web/package-lock.json .
RUN npm install
COPY web .

FROM web-src AS web-builder
RUN node_modules/.bin/webpack -p

FROM web-src AS web-test
RUN npm test

FROM nginx:1.19.1 AS web
COPY --from=web-builder /src/dist /usr/share/nginx/html