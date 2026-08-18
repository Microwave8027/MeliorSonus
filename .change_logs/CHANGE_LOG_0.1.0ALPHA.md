First changelog:
The ui works fine but the features are not actually implemented.
Actual rust integration has not been written, but uniffi has been bridged.
Cpal runs a audio engin which spawns a different worker thread for processing everything.
A supervisor thread checks for thread breaks and creates the worker thread
"Poisoning" within the worker thread is handled via a external public audio engine value

Agent wrote the implementation for polyphonic feature extraction and the nsdf determine algorithm
onnx tract runtime for crnn is not implemented
Monophonic feature extractor is finished and reviewed 

Goals for next changelog:
Review checking for polyphonic_feature extractor, clean it up a little bit. 

Goals for next major changelog:
Add a onnx runtime crnn or cnn of some kind that can run efficiently on mobile and provides conistant if not accurate frequency detections
Potentially FFI methods although if those are too difficult might move them to a later update
Write a lot more tests on functionality