FROM node:22-alpine
WORKDIR /oracle
COPY kafkajs/package.json ./
RUN npm install --omit=dev --no-audit --no-fund
COPY kafkajs/main.js ./
COPY wait-and-run.sh /usr/local/bin/wait-and-run.sh
ENTRYPOINT ["/usr/local/bin/wait-and-run.sh"]
