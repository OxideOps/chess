FROM catthehacker/ubuntu:rust-latest
LABEL org.opencontainers.image.source=https://github.com/OxideOps/chess

WORKDIR /root

ENV SERVER_FN_OVERRIDE_KEY=y

# Copy the setup script into the image
COPY setup.sh .

# Make the script executable and run it
RUN ./setup.sh -n
