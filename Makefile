.PHONY: install run clean

install:
	/usr/local/bin/python3 -m pip install Flask Flask-CORS APScheduler

run:
	/usr/local/bin/python3 app.py

clean:
	rm -rf data/*.db
	find . -name "__pycache__" -exec rm -rf {} +
