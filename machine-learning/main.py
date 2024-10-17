import cv2
import os
from insightface.app import FaceAnalysis
from scipy.spatial.distance import cosine

# FaceAnalysis is the same model used in Immich, its not free for comercial use but it serves for now as we are only testing
app = FaceAnalysis(name="buffalo_l", providers=['CPUExecutionProvider'])
app.prepare(ctx_id=0, det_size=(640, 640))  # ctx_id = 0 for CPU -1 for GPU

IMAGE_DIR = "C:\\Users\\Despacito4\\Desktop\\test_images"

output_folder = "identified_faces"
os.makedirs(output_folder, exist_ok=True)

face_data = []  # To store the embeddings - tuples (filename, face_index, embedding)

def process_image(file_path):

    img = cv2.imread(file_path)
    if img is None:
        print(f"Error reading - {file_path}")
        return
    
    faces = app.get(img) # Using the faceanalysis it already does the entire pipeline of detection and recognition and outputs both results 
    
    if len(faces) == 0:
        print(f"No faces found in - {file_path}") # We will finish the job here in these cases and store no info in the database, rows with null shall be those without people's faces
        return
    
    for idx, face in enumerate(faces):
        bbox = face.bbox.astype(int).flatten()  
        cv2.rectangle(img, (bbox[0], bbox[1]), (bbox[2], bbox[3]), (0, 255, 0), 2) # drawing a rectangle on found face (debugging)
        
        embedding = face.normed_embedding # face embedding vector (results from )
        
        face_data.append((file_path, idx, embedding)) # For now we are only saving the embeddings but we will need to save the location of one face for the preview generation
        
        print(f"Face {idx + 1} in {file_path}: {embedding[:5]}")
        
        cv2.putText(img, f"Face {idx + 1}", (bbox[0], bbox[1] - 10), cv2.FONT_HERSHEY_SIMPLEX, 0.8, (0, 255, 0), 2) # Labeling the face (debugging)
    
    output_path = os.path.join(output_folder, os.path.basename(file_path))
    cv2.imwrite(output_path, img)
    print(f"Success - saved result to {output_path}")


def compare_faces(embedding1, embedding2, threshold=0.5):
    # Compare cosine similarity between two embeddings, closer to 0 is more similar, the threshold might be too high, but I have no way of testing this for now
    similarity = cosine(embedding1, embedding2)
    return similarity < threshold


for filename in os.listdir(IMAGE_DIR):
    if filename.endswith((".jpg", ".jpeg", ".png")):  # we might need to expand this for other image formats like hevc or whatever apple uses
        file_path = os.path.join(IMAGE_DIR, filename)
        process_image(file_path)

# Compare faces between all stored embeddings
print("\n--- Face Recognition Results ---\n")
for i, (file1, face_idx1, embedding1) in enumerate(face_data):
    for j, (file2, face_idx2, embedding2) in enumerate(face_data):
        if i != j: 
            if compare_faces(embedding1, embedding2):
                print(f"Face {face_idx1 + 1} in {file1} matches Face {face_idx2 + 1} in {file2}")
