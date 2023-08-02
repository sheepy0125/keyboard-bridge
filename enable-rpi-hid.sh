#!/bin/bash
# A script to setup masquerading of the Raspberry Pi's OTG cable as a keyboard.
# Adapted from https://github.com/mtlynch/key-mime-pi#,
# which was adapted from https://github.com/girst/hardpass-sendHID/blob/master/README.md#.
# See https://www.kernel.org/doc/html/latest/usb/gadget_configfs.html

# Begin configuration =========================================================
MANUFACTURER="RPiKeyboardBridge"
PRODUCT="Bridged USB Keyboard"
SERIAL_NUMBER="1a2b3c4deee10213abc"
LANGUAGE="0x409"  # English (US) (https://web.archive.org/web/20000816171911/http://usb.org/developers/data/USB_LANGIDs.pdf)
VENDOR_ID="0x1d6b" # Linux Foundation
# End configuration   =========================================================

if [ "${EUID}" -ne 0 ]; then
    echo "Please run as root"
    exit 1
fi


set -e # Exit on first error
set -u # Treat undefined environment variables as errors.
modprobe libcomposite # Libcomposite is needed for USB gadget configfs (to setup the OTG master)

pushd /sys/kernel/config/usb_gadget/
mkdir -p g1
pushd g1

# Identification
echo "${VENDOR_ID}" > idVendor
echo 0x0104 > idProduct # Multifunction Composite Gadget
echo 0x0100 > bcdDevice # v1.0.0
echo 0x0200 > bcdUSB    # USB2
STRINGS_DIR="strings/${LANGUAGE}"
mkdir -p "${STRINGS_DIR}"
echo "${SERIAL_NUMBER}" > "${STRINGS_DIR}/serialnumber"
echo "${MANUFACTURER}" > "${STRINGS_DIR}/manufacturer"
echo "${PRODUCT}" > "${STRINGS_DIR}/product"
# Setup as keyboard
FUNCTIONS_DIR="functions/hid.usb0"
mkdir -p "$FUNCTIONS_DIR"
echo 1 > "${FUNCTIONS_DIR}/protocol" # Keyboard
echo 0 > "${FUNCTIONS_DIR}/subclass" # No subclass
echo 8 > "${FUNCTIONS_DIR}/report_length"
echo -ne `# Spoof as keyboard (https://www.kernel.org/doc/html/latest/usb/gadget_hid.html)`\
\\0x05\\0x01`# USAGE_PAGE (Generic Desktop)`\
\\0x09\\0x06`# USAGE (Keyboard)`\
\\0xa1\\0x01`# COLLECTION (Application)`\
\\0x05\\0x07`# USAGE_PAGE (Keyboard)`\
\\0x19\\0xe0`# USAGE_MINIMUM (Keyboard LeftControl)`\
\\0x29\\0xe7`# USAGE_MAXIMUM (Keyboard Right GUI)`\
\\0x15\\0x00`# LOGICAL_MINIMUM (0)`\
\\0x25\\0x01`# LOGICAL_MAXIMUM (1)`\
\\0x75\\0x01`# REPORT_SIZE (1)`\
\\0x95\\0x08`# REPORT_COUNT (8)`\
\\0x81\\0x02`# INPUT (Data,Var,Abs)`\
\\0x95\\0x01`# REPORT_COUNT (1)`\
\\0x75\\0x08`# REPORT_SIZE (8)`\
\\0x81\\0x03`# INPUT (Cnst,Var,Abs)`\
\\0x95\\0x05`# REPORT_COUNT (5)`\
\\0x75\\0x01`# REPORT_SIZE (1)`\
\\0x05\\0x08`# USAGE_PAGE (LEDs)`\
\\0x19\\0x01`# USAGE_MINIMUM (Num Lock)`\
\\0x29\\0x05`# USAGE_MAXIMUM (Kana)`\
\\0x91\\0x02`# OUTPUT (Data,Var,Abs)`\
\\0x95\\0x01`# REPORT_COUNT (1)`\
\\0x75\\0x03`# REPORT_SIZE (3)`\
\\0x91\\0x03`# OUTPUT (Cnst,Var,Abs)`\
\\0x95\\0x06`# REPORT_COUNT (6)`\
\\0x75\\0x08`# REPORT_SIZE (8)`\
\\0x15\\0x00`# LOGICAL_MINIMUM (0)`\
\\0x25\\0x65`# LOGICAL_MAXIMUM (101)`\
\\0x05\\0x07`# USAGE_PAGE (Keyboard)`\
\\0x19\\0x00`# USAGE_MINIMUM (Reserved)`\
\\0x29\\0x65`# USAGE_MAXIMUM (Keyboard Application)`\
\\0x81\\0x00`# INPUT (Data,Ary,Abs)`\
\\0xc0`#     # END_COLLECTION`\
> "${FUNCTIONS_DIR}/report_desc"
# Setup configurations
CONFIG_INDEX=1
CONFIGS_DIR="configs/c.${CONFIG_INDEX}"
mkdir -p "${CONFIGS_DIR}"
echo 250 > "${CONFIGS_DIR}/MaxPower"
CONFIGS_STRINGS_DIR="${CONFIGS_DIR}/strings/${LANGUAGE}"
mkdir -p "${CONFIGS_STRINGS_DIR}"
echo "Config ${CONFIG_INDEX}: USB keyboard bridge" > "${CONFIGS_STRINGS_DIR}/configuration"
ln -s "${FUNCTIONS_DIR}" "${CONFIGS_DIR}/"
ls /sys/class/udc > UDC

popd # from /sys/kernel/config/usb_gadget/g1
popd # from /sys/kernel/config/usb_gadget/

chmod 777 /dev/hidg0  # rwxrwxrwx:root