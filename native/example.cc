#include <iostream>
#include <iomanip>
#include <sstream>
#include <string>
#include <vector>

#include "native/api.h"

// Simple JSON helpers (no external library needed)
std::string json_escape(std::string const& s) {
    std::string result;
    for (char c : s) {
        switch (c) {
            case '"': result += "\\\""; break;
            case '\\': result += "\\\\"; break;
            case '\b': result += "\\b"; break;
            case '\f': result += "\\f"; break;
            case '\n': result += "\\n"; break;
            case '\r': result += "\\r"; break;
            case '\t': result += "\\t"; break;
            default: result += c;
        }
    }
    return result;
}

std::string coord_to_json(const motis::native::coord& c) {
    std::ostringstream oss;
    oss << "{\"lat\":" << c.lat << ",\"lon\":" << c.lon << "}";
    return oss.str();
}

std::string leg_to_json(const motis::native::leg& l) {
    std::ostringstream oss;
    oss << "{";
    oss << "\"mode\":\"" << json_escape(l.mode) << "\",";
    oss << "\"from_name\":\"" << json_escape(l.from_name) << "\",";
    oss << "\"to_name\":\"" << json_escape(l.to_name) << "\",";
    oss << "\"from\":" << coord_to_json(l.from) << ",";
    oss << "\"to\":" << coord_to_json(l.to) << ",";
    oss << "\"duration_seconds\":" << l.duration_seconds << ",";
    oss << "\"distance_meters\":" << l.distance_meters;
    if (l.route_short_name) {
        oss << ",\"route_short_name\":\"" << json_escape(*l.route_short_name) << "\"";
    }
    if (l.headsign) {
        oss << ",\"headsign\":\"" << json_escape(*l.headsign) << "\"";
    }
    oss << "}";
    return oss.str();
}

std::string route_to_json(const motis::native::route& r) {
    std::ostringstream oss;
    oss << "{";
    oss << "\"duration_seconds\":" << r.duration_seconds << ",";
    oss << "\"transfers\":" << r.transfers << ",";
    oss << "\"legs\":[";
    for (size_t i = 0; i < r.legs.size(); ++i) {
        if (i > 0) oss << ",";
        oss << leg_to_json(r.legs[i]);
    }
    oss << "]}";
    return oss.str();
}

std::string location_to_json(const motis::native::location& loc) {
    std::ostringstream oss;
    oss << "{";
    oss << "\"name\":\"" << json_escape(loc.name) << "\",";
    oss << "\"place_id\":\"" << json_escape(loc.place_id) << "\",";
    oss << "\"lat\":" << loc.pos.lat << ",";
    oss << "\"lon\":" << loc.pos.lon;
    if (loc.type) {
        oss << ",\"type\":\"" << json_escape(*loc.type) << "\"";
    }
    oss << "}";
    return oss.str();
}

void send_response(std::string const& data_json) {
    std::cout << "{\"status\":\"ok\",\"data\":" << data_json << "}" << std::endl;
}

void send_error(std::string const& msg) {
    std::cout << "{\"status\":\"error\",\"message\":\"" << json_escape(msg) << "\"}" << std::endl;
}

using namespace motis::native;

void run_ipc_mode(std::string const& data_path) {
    auto* inst = init(data_path);
    if (!inst) {
        send_error("Failed to initialize MOTIS");
        return;
    }
    
    std::string line;
    while (std::getline(std::cin, line)) {
        // Simple JSON parsing
        if (line.find("\"cmd\":\"geocode\"") != std::string::npos) {
            size_t pos = line.find("\"query\":\"");
            if (pos == std::string::npos) {
                send_error("Missing query parameter");
                continue;
            }
            pos += 9;
            size_t end = line.find("\"", pos);
            std::string query = line.substr(pos, end - pos);
            
            auto locations = geocode(*inst, query);
            std::ostringstream oss;
            oss << "[";
            for (size_t i = 0; i < locations.size(); ++i) {
                if (i > 0) oss << ",";
                oss << location_to_json(locations[i]);
            }
            oss << "]";
            send_response(oss.str());
        }
        else if (line.find("\"cmd\":\"plan_route\"") != std::string::npos) {
            // Extract coordinates with simple parsing
            auto get_double = [&](std::string const& key) -> double {
                size_t pos = line.find("\"" + key + "\":");
                if (pos == std::string::npos) return 0;
                pos += key.length() + 3;
                return std::stod(line.substr(pos, line.find(",", pos) - pos));
            };
            
            coord from{get_double("from_lat"), get_double("from_lon")};
            coord to{get_double("to_lat"), get_double("to_lon")};
            
            auto routes = plan_route(*inst, from, to);
            std::ostringstream oss;
            oss << "[";
            for (size_t i = 0; i < routes.size(); ++i) {
                if (i > 0) oss << ",";
                oss << route_to_json(routes[i]);
            }
            oss << "]";
            send_response(oss.str());
        }
        else if (line.find("\"cmd\":\"reverse_geocode\"") != std::string::npos) {
            auto get_double = [&](std::string const& key) -> double {
                size_t pos = line.find("\"" + key + "\":");
                if (pos == std::string::npos) return 0;
                pos += key.length() + 3;
                return std::stod(line.substr(pos, line.find(",", pos) - pos));
            };
            
            coord pos{get_double("lat"), get_double("lon")};
            auto loc = reverse_geocode(*inst, pos);
            
            if (loc) {
                send_response(location_to_json(*loc));
            } else {
                send_response("null");
            }
        }
        else {
            send_error("Unknown command");
        }
    }
    
    destroy(inst);
}

void print_route(const route& r) {
  std::cout << "Route: " << r.duration_seconds / 60 << " min, "
            << r.transfers << " transfers\n";
  
  for (auto const& leg : r.legs) {
    std::cout << "  [" << leg.mode << "] "
              << leg.from_name << " → " << leg.to_name;
    
    if (leg.route_short_name) {
      std::cout << " (" << *leg.route_short_name << ")";
    }
    std::cout << " - " << leg.duration_seconds / 60 << " min\n";
  }
  std::cout << "\n";
}

void run_demo_mode(std::string const& data_path) {
  std::cout << "Initializing MOTIS native API...\n";
  auto* inst = init(data_path);
  if (!inst) {
    std::cerr << "Failed to initialize MOTIS\n";
    return;
  }
  
  std::cout << "MOTIS loaded successfully!\n\n";
  
  // Example 1: Geocoding
  std::cout << "=== Geocoding: 'Stockholm Central' ===\n";
  auto locations = geocode(*inst, "Stockholm Central");
  for (auto const& loc : locations) {
    std::cout << "  " << loc.name << " ("
              << std::fixed << std::setprecision(4)
              << loc.pos.lat << ", " << loc.pos.lon << ")\n";
  }
  std::cout << "\n";
  
  // Example 2: Route planning
  std::cout << "=== Route Planning ===\n";
  coord stockholm = {59.3293, 18.0686};
  coord uppsala = {59.8586, 17.6389};
  
  std::cout << "From Stockholm to Uppsala:\n";
  auto routes = plan_route(*inst, stockholm, uppsala);
  
  if (routes.empty()) {
    std::cout << "  No routes found.\n";
  } else {
    std::cout << "  Found " << routes.size() << " route(s):\n\n";
    for (size_t i = 0; i < routes.size() && i < 3; ++i) {
      std::cout << "Route " << (i + 1) << ":\n";
      print_route(routes[i]);
    }
  }
  
  // Example 3: Reverse geocoding
  std::cout << "=== Reverse Geocoding ===\n";
  auto loc = reverse_geocode(*inst, stockholm);
  if (loc) {
    std::cout << "  " << loc->pos.lat << ", " << loc->pos.lon
              << " → " << loc->name << "\n";
  } else {
    std::cout << "  No result\n";
  }
  
  destroy(inst);
}

int main(int argc, char* argv[]) {
  if (argc < 2) {
    std::cerr << "Usage: " << argv[0] << " <data_path> [--ipc]\n\n"
              << "Options:\n"
              << "  --ipc    Run in IPC mode (JSON commands from stdin)\n\n"
              << "Examples:\n"
              << "  " << argv[0] << " ./data              # Demo mode\n"
              << "  " << argv[0] << " ./data --ipc        # IPC mode for GUI\n";
    return 1;
  }
  
  std::string data_path = argv[1];
  bool ipc_mode = (argc > 2 && std::string(argv[2]) == "--ipc");
  
  if (ipc_mode) {
    run_ipc_mode(data_path);
  } else {
    run_demo_mode(data_path);
  }
  
  return 0;
}
