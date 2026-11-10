# Roadmap

- Add a future `--details` mode for optional metadata, warnings, and deeper file inspection output.
- Hide TMATS-specific warning/detail output from the default view unless `--packets` or a future `--details` mode is used.
- Add terminal-width-aware output fallbacks based on detected console width.
- Add explicit output-layout parameters that can override automatic width detection.
- Add filtering to the packet listing so users can narrow the displayed packet rows.
- Add selection of an individual packet by its index in the recording for structural inspection.
- Add raw hexadecimal output grouped by packet header fields, without decoding payload contents.
- Include each packet's file offset in hexadecimal in the packet listing table.
