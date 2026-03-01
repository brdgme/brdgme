FROM node:22 AS web-src
WORKDIR /src
COPY web/package.json web/package-lock.json ./
RUN npm install
COPY web .

FROM web-src AS web-builder
RUN node_modules/.bin/webpack --mode production

FROM web-src AS web-test
RUN npm test

FROM nginx:stable AS web
COPY --from=web-builder /src/dist /usr/share/nginx/html
