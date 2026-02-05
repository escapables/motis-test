#pragma once

#include <string>
#include <vector>
#include <optional>
#include <memory>

namespace motis::native {

// Geo coordinate
struct coord {
  double lat;
  double lon;
};

// Simple route leg
struct leg {
  std::string mode;
  std::string from_name;
  std::string to_name;
  coord from;
  coord to;
  int duration_seconds;
  int distance_meters;
  std::optional<std::string> route_short_name;
  std::optional<std::string> headsign;
};

// Route result
struct route {
  int duration_seconds;
  int transfers;
  std::vector<leg> legs;
};

// Area for geocode result
struct area {
  std::string name;
  int admin_level;
  bool matched;
  bool unique;
  bool is_default;
};

// Token for matched text [start, length]
struct token {
  int start;
  int length;
};

// Geocode result with full Match data
struct location {
  std::string name;
  std::string place_id;
  coord pos;
  std::optional<std::string> type;  // "STOP", "PLACE", "ADDRESS"
  
  // Extended fields for full Match support
  std::vector<area> areas;          // Administrative areas (city, region, country)
  std::vector<token> tokens;        // Matched token positions
  double score;                     // Relevance score
  std::optional<std::string> category;  // POI category (for PLACE type)
  std::optional<std::vector<std::string>> modes;  // Transport modes (for STOP type)
  std::optional<double> importance; // Stop importance
  std::optional<std::string> street;
  std::optional<std::string> house_number;
  std::optional<std::string> country;
  std::optional<std::string> zip;
};

// Opaque handle to MOTIS instance
class native_instance;

// Initialize MOTIS native API
// Note: returns raw pointer to avoid unique_ptr with incomplete type issues
native_instance* init(const std::string& data_path);

// Cleanup
void destroy(native_instance* inst);

// Route planning
std::vector<route> plan_route(native_instance& inst,
                               coord from,
                               coord to,
                               std::optional<std::string> departure_time = std::nullopt);

// Geocoding
std::vector<location> geocode(native_instance& inst, const std::string& query);

// Reverse geocoding
std::optional<location> reverse_geocode(native_instance& inst, coord pos);

// Tile data (base64 encoded MVT)
struct tile_result {
  std::string data_base64;  // Base64 encoded tile data
  bool found;
};

// Get map tile (MVT format)
tile_result get_tile(native_instance& inst, int z, int x, int y);

// Glyph data (base64 encoded PBF)
struct glyph_result {
  std::string data_base64;
  bool found;
};

// Get glyph data from embedded SDF font resources.
glyph_result get_glyph(native_instance& inst, std::string const& path);

// Call supported MOTIS GET endpoints and return JSON payload.
// Input must be a path such as "/api/v1/stoptimes?stopId=...".
std::optional<std::string> api_get(native_instance& inst,
                                   std::string const& path_and_query);

}  // namespace motis::native
