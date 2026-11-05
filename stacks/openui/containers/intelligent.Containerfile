FROM docker.io/library/node:24-bookworm-slim@sha256:c86ecead9e55a5210ac1727de38a4ca840bf03dcaf34dc9749addbf383b1920c AS build
WORKDIR /app
ENV NEXT_TELEMETRY_DISABLED=1 DO_NOT_TRACK=1
COPY sources/intelligent/ ./
COPY apps/intelligent/ ./
RUN npm ci --no-audit --no-fund && npm run generate && npm run prompt && npm run build
FROM docker.io/library/node:24-bookworm-slim@sha256:c86ecead9e55a5210ac1727de38a4ca840bf03dcaf34dc9749addbf383b1920c
WORKDIR /app
ENV NODE_ENV=production NEXT_TELEMETRY_DISABLED=1
COPY --from=build --chown=node:node /app /app
USER node
EXPOSE 3000
CMD ["node","node_modules/next/dist/bin/next","start","-H","0.0.0.0","-p","3000"]
