FROM node:8.6-alpine

WORKDIR /app
COPY package.json /app

COPY . /app

ENTRYPOINT ["npm", "run", "start:staging"]
