FROM node:16.16.0-alpine

WORKDIR /app
COPY package.json pnpm-lock.yaml /app/

RUN pnpm install

COPY . /app

ENTRYPOINT ["pnpm", "start:staging"]
