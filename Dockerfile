FROM registry.suse.com/bci/rust:latest as build
RUN zypper -n in openssl-devel
WORKDIR /app/
COPY . .

RUN cargo build

# FROM registry.suse.com/bci/bci-minimal:latest
# RUN zypper -n in openssl-devel

# COPY --from=build /app/target/debug/autocluster .