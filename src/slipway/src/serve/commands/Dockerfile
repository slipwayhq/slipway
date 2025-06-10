FROM slipwayhq/slipway:latest

WORKDIR /usr/artifacts
COPY ./slipway_serve.json ./slipway_serve.json
COPY ./devices ./devices
COPY ./playlists ./playlists
COPY ./rigs ./rigs
COPY ./components ./components
COPY ./fonts ./fonts

# Configure the timezone in the image.
RUN set -eux; \
    # Extract the timezone from the JSON config file
    zone="$(jq -r '.timezone' slipway_serve.json)"; \
    # sanity-check it exists in tzdata
    [ -f "/usr/share/zoneinfo/$zone" ]; \
    ln -snf "/usr/share/zoneinfo/$zone" /etc/localtime; \
    echo "$zone" > /etc/timezone; \
    # Tidy up
    apt-get purge -y --auto-remove jq && rm -rf /var/lib/apt/lists/*

RUN slipway serve . aot-compile

CMD ["slipway", "serve", ".", "--aot"]
