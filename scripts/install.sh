#!/bin/sh
#
# Copyright (c) 2024 Damián Sánchez Moreno
#
# This program is free software: you can redistribute it and/or modify it under
# the terms of the GNU General Public License as published by the Free Software
# Foundation, either version 3 of the License, or (at your option) any later
# version.
#
# This program is distributed in the hope that it will be useful, but WITHOUT
# ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
# FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License along with
# this program. If not, see <https://www.gnu.org/licenses/>.
#



REMARKABLE_HOST="remarkable"
LOCAL_BIN_DIR=".local/bin"
REPOCKET_DIR=".local/share/repocket/"
AUTH_FILE="data/.repocket.key"
BIN_FILE="target/armv7-unknown-linux-gnueabihf/release/rePocket"
SERVICE_FILE="rePocket/repocket.service"


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

# Copy the service and initialize it
echo "Copying the service file"
TARGET_DIR="/etc/systemd/system/"
scp "${SERVICE_FILE}" "${TARGET_DIR}"

echo "Enable and initialize the service"
ssh remarkable systemctl daemon-reload
ssh remarkable systemctl enable repocket.service
ssh remarkable systemctl start repocket.service
