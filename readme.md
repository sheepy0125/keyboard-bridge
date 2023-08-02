# Keyboard bridge

A bridge to run on my Raspberry Pi to transmit keyboard events over to the OTG cable. USB on-the-go or OTG allows it to act as a master rather than a slave, and thusly masquerade as a keyboard.  
My use case for this is for this to be used in conjunction with [Kanata](https://github.com/jtroo/kanata/) to handle keyboard layers and layouts. Thusly, even on a locked down computer, I can use another keyboard layout and have all my layers.

## Features

-   [ ] Functions as a keyboard bridge
-   [ ] Keylogger (if so desired)

## Usage and installation

### Exiting

Press `<Enter>` `~` `.` `<Backspace>` `<Backspace>` `<Backspace>` `<Enter>` to exit.  
Note: You must hold `Shift` after Enter to get `~`, not before.

### Prerequisites

-   The Raspberry Pi plugged into a computer through a USB C cable with data lines
-   A keyboard plugged into the Raspberry Pi that Linux recognizes
-   Rust and Cargo (https://rustup.rs/)
-   A Linux kernel with `libcomposite` available as a kernel module (most likely is)

### Setup

1. Enable the RPi USB OTG as a keyboard device.
    ```bash
    # In project root
    chmod +x ./enable-rpi-hid.sh
    sudo ./enable-rpi-hid.sh
    sudo reboot
    ```
1. Start the bridge once to ensure it works
    ```bash
    # In project root
    chmod +x ./run.sh
    ./run.sh
    ```
1. Plug in the keyboard and start typing
1. Exit (see [Exiting](#exiting))

### Autostart

1. Enable autologin for your user (`sudo raspi-config`, `1 System Options` -> `S5 Boot / Auto Login` -> `B2 Console Autologin`)
1. Add the following to your `.bashrc` (or any other shell's runcom/autostart file if used)
    ```bash
    if [ "$(tty)" = "/dev/tty1" ] && [ "$TERM" = "linux" ]; then if [ -n "$(pgrep keyboard-bridge)" ]; then :; else
        keyboard-bridge
    fi fi
    ```

## How it works

This program "grabs" all real keyboards connected in `/dev/input`, where grabbing means retaining exclusive access so that no key events can be sent to any other programs. Then, it maps the key events taken in to [valid keyboard USB events](https://www.usb.org/sites/default/files/documents/hid1_11.pdf).

The considerations in intercepting `/dev/input` events are as follows:

-   Itercepting the raw input from an actual USB keyboard and just piping the input to the OTG output wouldn't allow for another program to do key interception and remapping
-   Using `stdin` does not allow for special and modifier keys (e.g. page up and control+shift) to be received.
-   Using X.Org or Wayland to handle key events would be bloated

However, I found it easier to have my Pi autologin to my user and launch this in `.bashrc`, especially since there's no need to either hook into Kanata and I don't need the keyboard to send any events.

## Key Mime Pi

Thanks to Michael Lynch for creating [Key Mime Pi](https://mtlynch.io/key-mime-pi/) (I like the pun), a tool designed to transfer keypresses between computers over WebSockets with a Flask webpage. His project was excellent in terms of documentation.
