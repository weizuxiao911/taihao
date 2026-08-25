FROM node:20-alpine

WORKDIR /app

COPY package*.json ./
RUN npm install

COPY tsconfig.json ./
COPY src ./src
RUN npm run build

ENV PORT=80
ENV HOST=0.0.0.0

EXPOSE 80

CMD ["node", "dist/index.js"]
