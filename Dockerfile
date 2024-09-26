# Use the official Rust image as the base image
FROM rust:latest

# Install NASM
RUN apt-get update && \
    apt-get install -y nasm && \
    apt-get clean

# Set a working directory within the container
WORKDIR /usr/src/app

# Copy Cargo.toml and Cargo.lock to cache dependencies
COPY Cargo.toml Cargo.lock ./

# Fetch dependencies
RUN cargo fetch

# Copy the entire project source code to the container
COPY . .

# Build the dependencies and application
#RUN cargo build --release
#RUN cargo build --release --bin avif_converter_server && ls -al ./target/release
RUN cargo build --release --bin main && ls -al ./target/release

#RUN ls -al ./target/release

#RUN ls -al ./target/release/avif_converter_server

#RUN chmod +x ./target/release/avif_converter_server

# Specify the command to run your Rust application
#CMD ["./target/release/avif_converter_server"]
CMD ["./target/release/main"]
