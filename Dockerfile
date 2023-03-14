FROM rust:1.67

WORKDIR /app
COPY ./target/release/rust-eutils /app
COPY ./web /app/web

EXPOSE 4321

CMD ["./rust-eutils"]