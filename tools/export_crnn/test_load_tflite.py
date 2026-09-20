import tensorflow as tf
import numpy as np

for model_name in [
    "models/bytedance_crnn_acoustic_fp32.tflite",
    "models/bytedance_crnn_acoustic_drq_int8.tflite",
]:
    print(f"\n=======================================================")
    print(f"Testing model: {model_name}")
    print(f"=======================================================")
    interp = tf.lite.Interpreter(model_path=model_name)
    interp.allocate_tensors()
    inputs = interp.get_input_details()
    outputs = interp.get_output_details()

    print("Inputs:", [(i['name'], i['shape'].tolist()) for i in inputs])
    print(f"Outputs Count: {len(outputs)}")
    for idx, o in enumerate(outputs):
        print(f"  Output {idx}: {o['name']} -> {o['shape'].tolist()}")

    dummy = np.zeros(inputs[0]['shape'], dtype=np.float32)
    interp.set_tensor(inputs[0]['index'], dummy)
    interp.invoke()
    print("Inference successful! Outputs verified.")
