import requests
import json

# The URL of the API endpoint
url = "https://eloeverything.co/api/items"

# Headers for the POST request
headers = {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:113.0) Gecko/20100101 Firefox/113.0",
    "Accept": "application/json, text/plain, */*",
    "Accept-Language": "en-US,en;q=0.5",
    "Accept-Encoding": "gzip, deflate, br",
    "Content-Type": "application/json",
    "Origin": "https://eloeverything.co",
    "Connection": "keep-alive",
    "Referer": "https://eloeverything.co/leaderboard",
    # Add your cookie here
    "Cookie": "__Host-next-auth.csrf-token=acd01435ef6447581379400677fb47dac5d4e6234a609bf93673d3962e5041a2%7C136b04db7d137f4ccb0f0c80a720850df3b2a8c9fc145a5873f0a9d340e6f27a; __Secure-next-auth.callback-url=https%3A%2F%2Feloeverything.co; _ga_MN7QPTQZR1=GS1.1.1705346109.1.1.1705347688.0.0.0; _ga=GA1.1.1438003416.1705346109; userId=tvPCWwEyb1yz" 
}

# JSON body for the POST request (if required)
data = {}

# Make the POST request
response = requests.post(url, headers=headers, json=data)

# Check if the request was successful
if response.status_code == 200:
    # Parse the JSON response
    items = response.json()

    # Extract and print the required information
    for item in items:
        print(f"Name: {item['name']}, Wikipedia Link: {item['wikipedia']}")
else:
    print(f"Failed to retrieve data. Status code: {response.status_code}, Response: {response.text}")

