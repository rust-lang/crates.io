FROM node:8.6-alpine

WORKDIR /app
COPY package.json /app

RUN yarn install

COPY . /app

ENTRYPOINT ["yarn", "run", "start:staging"]
