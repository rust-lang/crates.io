FROM node:8.6-alpine

WORKDIR /app
COPY package.json /app

COPY . /app

RUN npm install -g ember-cli
RUN npm install

ENTRYPOINT ["npm", "run", "start:staging"]
