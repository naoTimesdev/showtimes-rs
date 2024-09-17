FROM rust:latest

WORKDIR /showtimes
COPY . .

RUN cargo build --locked --profile production --bin showtimes

EXPOSE 5560

CMD ["./target/production/showtimes"]
