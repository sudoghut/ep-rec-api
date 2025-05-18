# EP-REC-API

## Building with Docker

You can build a release binary within a Fedora Docker container:

1.  **Build Image:** `docker build -t ep-rec-api .`
2.  **Create Container:** `docker create --name ep-rec-api-container ep-rec-api`
3.  **Copy Binary:** `docker cp ep-rec-api-container:/app/target/release/ep-rec-api .`
4.  **Cleanup (Optional):** `docker rm ep-rec-api-container`
5.  **Remove Image (Optional):** `docker rmi ep-rec-api`

Now you have the `ep-rec-api` binary built in your host server, ready to run on a compatible system.

## Installation on Linux

1. Clone the repository:
    ```bash
    git clone https://github.com/sudoghut/ep-rec-api
    ```

2.  **Create a systemd Unit File:**
    Create a file named `ep-rec-api.service` in `/etc/systemd/system/` with the following content. **Change the following values** for your actual settings.

    ```ini
    [Unit]
    Description=Ep-rec-api Server
    After=network.target

    [Service]
    User=linuxuser
    Group=linuxuser
    WorkingDirectory=/home/linuxuser/ep-rec-api
    ExecStart=/usr/bin/env /home/linuxuser/ep-rec-api/ep-rec-api
    Restart=on-failure
    StandardOutput=journal
    StandardError=journal

    [Install]
    WantedBy=multi-user.target
    ```

3.  **Enable and Start the Service:**
    ```bash
    # Reload systemd to recognize the new service file
    sudo systemctl daemon-reload

    # Enable the service to start on boot
    sudo systemctl enable ep-rec-api.service

    # Start the service immediately
    sudo systemctl start ep-rec-api.service