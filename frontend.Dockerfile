FROM node:16.13.2-alpine

WORKDIR /app
COPY package.json yarn.lock /app/

RUN yarn install

COPY . /app

ENTRYPOINT ["yarn", "start:staging"]
