FROM ubuntu:xenial
ENV ROCKET_ENV prod
RUN apt-get update && apt-get install -y libpq-dev
ADD brdgme-api brdgme-api
ENTRYPOINT ./brdgme-api
EXPOSE 80
