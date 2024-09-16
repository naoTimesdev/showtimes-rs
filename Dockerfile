FROM rust:latest

WORKDIR /showtimes
COPY . .

RUN cargo build --locked --release --bin showtimes

CMD ["./target/release/showtimes"]
