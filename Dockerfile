# Latest stable Alpine release has rust 1.56.1 which is too old
FROM alpine:20220328

# ripgrep and git are needed by jxr-backend
RUN apk add --no-cache \
  cargo \
  ripgrep \
  git

WORKDIR /jxr-backend
COPY . .
RUN cargo build

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=80
ENV JXR_CODE_DIR=/jxr-indexed-code
CMD ["cargo", "run"]
EXPOSE 80