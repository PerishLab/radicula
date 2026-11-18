# radicula-socket

Blocking TCP client connections over the standard library, with no third-party
dependency and no protocol knowledge. This is an independent library root; it
depends on no other PerishLab library.

The connect timeout is one deadline shared by every resolved address. Name
resolution uses the standard resolver, which cannot be bounded, so it is not
covered by that timeout. Read and write limits apply to a single operation and
restart if a call is interrupted by a signal. A zero duration is refused rather
than treated as unlimited.
