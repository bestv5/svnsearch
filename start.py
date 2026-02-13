#!/usr/bin/env python3
import subprocess
import sys
import os

def install_dependencies():
    subprocess.check_call([sys.executable, "-m", "pip", "install", "-q", 
                          "Flask", "Flask-CORS", "APScheduler"])

def main():
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    install_dependencies()
    
    from app import app
    print("\n" + "="*50)
    print("  SVN Search Server Started!")
    print("  Open http://localhost:5000 in your browser")
    print("="*50 + "\n")
    app.run(host='0.0.0.0', port=5000, debug=False)

if __name__ == "__main__":
    main()
