FROM node:17.1.0-alpine

WORKDIR /app
COPY package.json yarn.lock /app/

RUN yarn install

COPY . /app

ENTRYPOINT ["yarn", "start:staging"]
