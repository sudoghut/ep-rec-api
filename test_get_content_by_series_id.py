import requests
import json

url = "http://127.0.0.1:3000/get_content_by_series_id"
headers = {
    "Content-Type": "application/json"
}
data = {
    "id_list": [13]
}
response = requests.post(url, headers=headers, data=json.dumps(data))
if response.status_code == 200:
    print("Response:")
    print(json.dumps(response.json(), indent=4, ensure_ascii=False))
else:
    print("Error:", response.status_code)
    print("Response:", response.text)
