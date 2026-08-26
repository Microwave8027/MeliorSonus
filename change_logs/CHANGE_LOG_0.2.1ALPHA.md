Changelog:
Changed the time tracking for systemtime which relies on cpu and can be unreliable to the amount of frames getting passed in depending on the config
Added a striked notes so that when a note is striked or when it is detected it is pushed into the ring buffer instead of waiting. This allows a more robust and cleaner audio analyzer later on

Also refactored the entire codebase to use specific rs files that mod and export entire folders rather then individually writing a paths.rs which may allow issues