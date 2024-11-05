#!/bin/sh

REMARKABLE_HOST="remarkable"
LOCAL_BIN_DIR=".local/bin"
REPOCKET_DIR=".local/share/repocket/"
AUTH_FILE="data/.repocket.key"
BIN_FILE="target/armv7-unknown-linux-gnueabihf/release/rePocket"


echo "Create the necessary folders.."
ssh ${REMARKABLE_HOST} "mkdir -p ${LOCAL_BIN_DIR}"
ssh ${REMARKABLE_HOST} "mkdir -p ${REPOCKET_DIR}"


# Copy the auth file
echo "Copying files.."
TARGET_DIR="${REMARKABLE_HOST}:${REPOCKET_DIR}"
scp "${AUTH_FILE}" "${TARGET_DIR}"

# Copy the binary
TARGET_DIR="${REMARKABLE_HOST}:${LOCAL_BIN_DIR}"
scp "${BIN_FILE}" "${TARGET_DIR}"
