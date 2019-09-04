FROM node:12.9-alpine

WORKDIR /app
COPY package.json package-lock.json /app/

RUN npm install

COPY . /app

ENTRYPOINT ["npm", "run", "start:staging"]
