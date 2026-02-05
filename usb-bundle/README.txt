MOTIS Transit - Portable USB Edition
====================================

WHAT IS THIS?
A portable public transit planning application that runs entirely
from a USB stick. No installation required!

QUICK START:
1. Copy your MOTIS data to the 'data/' folder
   (or run the setup steps below)

2. Run the native GUI (recommended):
   ./motis-gui
   
   Or use the web interface:
   ./start.sh
   Open http://localhost:8080 in your browser

GUI DEBUG MODE:
---------------
To see the debug console in the GUI, run with --debug flag:

    ./motis-gui --debug

This shows technical logs useful for troubleshooting.

WEB INTERFACE:
--------------
This bundle includes the full MOTIS web UI in the 'ui/' folder.
The start.sh script automatically configures the server to use it.

If you see a blank page:
  - Check that the 'ui/' folder exists and has files
  - Try refreshing the page
  - Check browser console for errors

The server API also works directly:
    
    curl "http://localhost:8080/api/v1/geocode?text=Central"
    curl "http://localhost:8080/api/v1/plan?fromPlace=59.33,18.06&toPlace=59.86,17.64"

SETUP - Creating Data for Your Region:
--------------------------------------
You need two files:
1. GTFS feed (transit schedules) - from your local transit agency
2. OSM extract (.osm.pbf) - from openstreetmap.org

Steps:
  1. Place files in this directory
  2. Run: ./motis-transit config your_region.osm.pbf your_gtfs.zip
  3. Run: ./motis-transit import
  4. Run: ./start.sh

USB PORTABILITY:
- All data stored in ./data/ folder
- No registry entries
- No system dependencies
- Works on any x86_64 Linux system

TROUBLESHOOTING:
- Port 8080 already in use? Edit start.sh to use different port
- Missing libraries? Install: libssl1.1 (for OpenSSL 1.1 compatibility)
- Blank page? The web UI may not be included. Use API directly with curl.

API EXAMPLES:
  # Geocode
  curl "http://localhost:8080/api/v1/geocode?text=Stockholm%20Central"
  
  # Route planning
  curl "http://localhost:8080/api/v1/plan?fromPlace=59.3293,18.0686&toPlace=59.8586,17.6389"

MORE INFO:
https://github.com/motis-project/motis
https://github.com/escapables/motis-portable
