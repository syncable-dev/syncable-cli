# Test Dockerfile with maximum hadolint errors
# This file is intentionally bad - DO NOT use in production!

# DL4000: MAINTAINER is deprecated
MAINTAINER bad-practice@example.com

# DL3006: Image without tag
# DL3007: Using :latest tag (pick one - this uses latest)
FROM ubuntu:latest

# DL3049: maintainer label is deprecated
# DL3050: Superfluous labels (should use OCI format)
# DL3051: Invalid RFC3339 date
# DL3052: Invalid SPDX license
# DL3053: Empty title
# DL3054: Empty description
# DL3055: Invalid documentation URL
# DL3056: Invalid source URL
# DL3058: Invalid image URL
LABEL maintainer="test@test.com" \
      description="" \
      title="" \
      version="1.0" \
      org.opencontainers.image.created="not-a-date" \
      org.opencontainers.image.licenses="INVALID-LICENSE" \
      org.opencontainers.image.title="" \
      org.opencontainers.image.description="" \
      org.opencontainers.image.documentation="not-a-url" \
      org.opencontainers.image.source="also-not-a-url" \
      org.opencontainers.image.url="still-not-a-url"

# DL3044: ENV self-reference in same statement
ENV FOO=bar BAR=$FOO

# DL3045: COPY to relative destination without WORKDIR
COPY package.json app/

# DL3000: Use absolute WORKDIR
WORKDIR relative/path

# DL3005: Don't use apt-get upgrade
# DL3008: Pin versions in apt-get install
# DL3009: Delete apt-get lists after install
# DL3014: Use -y with apt-get install
# DL3015: Avoid additional packages with --no-install-recommends
# DL3027: Don't use apt, use apt-get
# DL3059: Multiple consecutive RUN instructions
RUN apt update
RUN apt-get upgrade
RUN apt-get install curl wget nginx vim ssh

# DL3004: Don't use sudo
# DL3046: useradd without -l flag
RUN sudo useradd -m testuser

# DL3003: Use WORKDIR instead of cd
RUN cd /app && echo "hello"

# DL3001: For commands like vim, ssh not recommended
RUN ssh-keygen -t rsa -N "" -f /root/.ssh/id_rsa

# DL3013: Pin pip versions
# DL3042: Use --no-cache-dir with pip
RUN pip install flask requests numpy

# DL3016: Pin npm versions
RUN npm install -g express lodash

# DL3028: Pin gem versions
RUN gem install rails sinatra

# DL3060: yarn cache clean missing
RUN yarn add react redux

# DL3017: Don't use apk upgrade
# DL3018: Pin apk versions
# DL3019: Use --no-cache for apk add
FROM alpine:latest AS alpine-stage
RUN apk upgrade
RUN apk add nginx curl

# DL3031: Don't use yum update
# DL3032: yum clean all missing
# DL3033: Pin yum versions
FROM centos:latest AS centos-stage
RUN yum update -y
RUN yum install -y httpd

# DL3035: Don't use zypper update
# DL3036: zypper clean missing
# DL3037: Pin zypper versions
# DL3030: Use -n for zypper install
# DL3034: Use -n for zypper
FROM opensuse/leap:latest AS suse-stage
RUN zypper update
RUN zypper install apache2

# DL3039: Don't use dnf update
# DL3040: dnf clean missing
# DL3041: Pin dnf versions
# DL3038: Use -y with dnf install
FROM fedora:latest AS fedora-stage
RUN dnf update
RUN dnf install nginx

# DL3024: FROM aliases must be unique (duplicate alias)
FROM ubuntu:latest AS builder
FROM debian:latest AS builder

# DL3020: Use COPY instead of ADD for files
# DL3021: Use COPY instead of ADD for non-archives
ADD https://example.com/file.txt /app/
ADD localfile.txt /app/

# DL3010: Use ADD for extracting archives (this uses COPY which is wrong)
COPY archive.tar.gz /app/

# DL3022: COPY --from references undefined stage
# DL3062: COPY --from undefined stage
COPY --from=nonexistent /app /app

# DL3023: COPY --from cannot reference own stage
COPY --from=builder /build /app

# DL3011: Invalid UNIX port
EXPOSE 99999
EXPOSE -1

# DL3043: ONBUILD ONBUILD is not allowed
ONBUILD ONBUILD RUN echo "nested onbuild"

# DL4005: Use SHELL to change shell (instead of ln -s)
RUN ln -s /bin/bash /bin/sh

# DL4006: Set SHELL for pipefail
RUN curl http://example.com | grep pattern

# DL3047: wget vs curl - using both
# DL4001: Either use wget or curl, not both
RUN wget http://example.com/file1
RUN curl http://example.com/file2

# DL3025: Use JSON form for CMD/ENTRYPOINT
ENTRYPOINT /bin/bash -c "start.sh"

# DL4003: Multiple CMD instructions
CMD echo "first cmd"
CMD echo "second cmd"

# DL4004: Multiple ENTRYPOINT instructions
ENTRYPOINT ["python"]
ENTRYPOINT ["node"]

# DL3012: Multiple HEALTHCHECK instructions
HEALTHCHECK CMD curl -f http://localhost/
HEALTHCHECK CMD wget -q http://localhost/

# DL3002: Last USER should not be root
USER root