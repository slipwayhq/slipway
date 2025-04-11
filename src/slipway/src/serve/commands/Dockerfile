FROM slipwayhq/slipway:latest

WORKDIR /usr/artifacts
COPY ./slipway_serve.json ./slipway_serve.json
COPY ./devices ./devices
COPY ./playlists ./playlists
COPY ./rigs ./rigs
COPY ./components ./components
COPY ./fonts ./fonts

RUN slipway serve . aot-compile --target x86_64-unknown-linux-gnu

CMD ["slipway", "serve", ".", "--aot"]
