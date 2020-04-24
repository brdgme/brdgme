FROM node:lts-alpine
WORKDIR /app
COPY src ./src
COPY test ./test
COPY package.json package-lock.json tsconfig.json tslint.json webpack.config.js ./
RUN npm install
RUN npm run test
RUN npm run build

FROM nginx:latest
COPY --from=0 /app/dist /usr/share/nginx/html