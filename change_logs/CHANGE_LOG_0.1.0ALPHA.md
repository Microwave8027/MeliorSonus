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

Mermaid digram: 


flowchart TD
        subgraph MAIN_THREAD ["Main Thread / Initialization (One-Time Startup)"]
            A["Create Ring Buffers: HeapRb::split()"] --> B["HeapProd (Producers) & HeapCons (Consumers)"]
            B --> C["Instantiate PolyphonicFeatureExtractorImpl"]

            subgraph EMBEDDED_SINGLETONS ["Nested Pre-allocated Singletons"]
                C --> C1["NoteFeatureExtractorImpl (Fast-Path MPM)"]
                C --> C2["HarmonicSieveMasker (88 Bin Centers & Ghost Mask)"]
                C --> C3["NsdfEvaluator (NSDF Scratchpad)"]
            end

            C --> D["AudioEngine::new(Instrument, poly_extractor)"]
            D --> E["engine.play()"]
        end

        subgraph SUPERVISOR_THREAD ["CPAL Audio Supervisor Thread (cpal.rs)"]
            E --> F["Build Stream & Instantiate Stream Singletons"]
            F --> F1["BandPassFilter (IIR)"]
            F --> F2["MPM (PitchDetector)"]
            F --> F3["HeapCons<f32> Hardware Buffer"]

            F3 --> G{"Buffer >= 1024 samples?"}
            G -- Yes --> H["Invoke dsp_callback(CallBackParameters)"]
            G -- No --> G
        end

        subgraph AUDIO_CALLBACK ["HOT-PATH AUDIO CALLBACK (dsp_callback) - ZERO HEAP ALLOCATIONS"]
            H --> I["High-Pass / Band-Pass Filter (BandPassFilter)"]
            I --> J["Compute Loudness (RMS dBFS)"]
            J --> K{"dBFS < Threshold?"}

            K -- Yes (Silence) --> L["Mode = Silence (Idle)"]
            K -- No (Active Signal) --> M["NSDF Evaluation: NsdfEvaluator::evaluate_frame()"]

            M --> N["Extract Clarity (r1) & Secondary Peak Ratio (r2/r1)"]
            N --> O{"is_monophonic? (r1 >= instrument_thresh)"}

            %% Fast Path
            O -- Yes (High Clarity) --> P["FAST-PATH MONOPHONIC"]
            P --> P1["CRNN Sleep Mode (0% CPU)"]
            P --> P2["Delegate to NoteFeatureExtractorImpl (MPM)"]
            P2 --> P3["Single Note State Machine: Rise -> Peak -> Decay"]

            %% Polyphonic Path
            O -- No (Polyphonic Mixture) --> Q["POLYPHONIC CRNN + SIEVE PATH"]
            Q --> Q1["Log-CQT Spectrogram Feature Window"]
            Q1 --> Q2["tract-onnx ONNX Inference (run_tract_inference)"]
            Q2 --> Q3["HarmonicSieveMasker: Ghost Harmonic Masking"]
            Q3 --> Q4["Multi-Note State Array: RecordNote Duration Tracking"]

            P3 --> R{"Note Finalized / Released?"}
            Q4 --> R

            R -- Yes --> S["push (Note, timestamp) into HeapProd"]
            R -- No --> T["Wait for Next Audio Frame (512 Hop)"]
        end

        subgraph UI_CONSUMER ["UI / Kotlin / Swift Consumer Thread"]
            S --> U["Read Emitted Notes from HeapCons Ring Buffer"]
        end

        classDef mainStyle fill:#1a202c,stroke:#4a5568,color:#fff
        classDef superStyle fill:#2d3748,stroke:#4a5568,color:#fff
        classDef callbackStyle fill:#1a365d,stroke:#2b6cb0,color:#fff
        classDef uiStyle fill:#276749,stroke:#38a169,color:#fff

        class A,B,C,C1,C2,C3,D,E mainStyle
        class F,F1,F2,F3,G superStyle
        class H,I,J,K,L,M,N,O,P,P1,P2,P3,Q,Q1,Q2,Q3,Q4,R,S,T callbackStyle
        class U uiStyle
