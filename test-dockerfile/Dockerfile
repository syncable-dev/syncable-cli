# Test Dockerfile with various hadolint rule violations
# This file is intentionally problematic for testing purposes

FROM ubuntu
MAINTAINER John Doe <john@example.com>

# Using sudo (DL3004) and apt instead of apt-get (DL3027)
RUN sudo apt update

# Unpinned apt-get packages (DL3008)
RUN apt-get update && apt-get install -y \
    nginx \
    curl \
    vim \
    wget

# Using cd instead of WORKDIR (DL3003)
RUN cd /app && npm install

# Unpinned pip packages (DL3013)
RUN pip install flask requests numpy pandas

# Relative WORKDIR (DL3000)
WORKDIR app

# Multiple CMD instructions (DL4003)
CMD echo "first"
CMD echo "second"

# Shell form CMD instead of exec form (DL3025)
CMD node server.js

# Using latest tag (DL3007)
FROM node:latest AS builder
WORKDIR /build
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

# Good stage - should pass most rules
FROM node:18-alpine AS production
WORKDIR /app
COPY --from=builder /build/dist ./dist
USER node
EXPOSE 3000
CMD ["node", "dist/index.js"]

# Another stage with issues
FROM python:3.11
# No WORKDIR set, copying to relative path (DL3045)
COPY requirements.txt .
COPY app/ app/

# Pipe without pipefail (DL4006)
RUN curl -sL https://example.com/script.sh | bash

# Multiple ENTRYPOINT (DL4004)
ENTRYPOINT ["python"]
ENTRYPOINT ["python", "app/main.py"]

# ADD for local file (should use COPY) (DL3020)
ADD config.json /etc/app/config.json

# Ending as root (DL3002)
USER root
RUN apt-get update && apt-get install -y openssh-server
