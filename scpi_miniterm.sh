if [[ $# -lt 3 ]]; then
  echo "Usage: $0 [options] <port> <baudrate>"
  exit 1
fi

python3 -m serial.tools.miniterm --eol LF --echo "$*"
