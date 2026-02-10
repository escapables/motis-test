#include <iostream>
#include <iomanip>
#include <sstream>
#include <string>
#include <vector>

#include "native/api.h"
#include <nlohmann/json.hpp>

using json = nlohmann::json;
using namespace motis::native;

json coord_to_json(const coord& c) {
    return json{{"lat", c.lat}, {"lon", c.lon}};
}

json leg_to_json(const leg& l) {
    json j = {
        {"mode", l.mode},
        {"from_name", l.from_name},
        {"to_name", l.to_name},
        {"from", coord_to_json(l.from)},
        {"to", coord_to_json(l.to)},
        {"duration_seconds", l.duration_seconds},
        {"distance_meters", l.distance_meters}
    };
    if (l.route_short_name) {
        j["route_short_name"] = *l.route_short_name;
    }
    if (l.headsign) {
        j["headsign"] = *l.headsign;
    }
    return j;
}

json route_to_json(const route& r) {
    json legs = json::array();
    for (auto const& l : r.legs) {
        legs.push_back(leg_to_json(l));
    }
    return json{
        {"duration_seconds", r.duration_seconds},
        {"transfers", r.transfers},
        {"legs", legs}
    };
}

json area_to_json(const area& a) {
    return json{
        {"name", a.name},
        {"admin_level", a.admin_level},
        {"matched", a.matched},
        {"unique", a.unique},
        {"default", a.is_default}
    };
}

json token_to_json(const token& t) {
    return json::array({t.start, t.length});
}

json location_to_json(const location& loc) {
    json j = {
        {"name", loc.name},
        {"place_id", loc.place_id},
        {"lat", loc.pos.lat},
        {"lon", loc.pos.lon},
        {"score", loc.score}
    };
    
    if (loc.type) {
        j["type"] = *loc.type;
    }
    if (loc.category) {
        j["category"] = *loc.category;
    }
    
    // Areas
    json areas = json::array();
    for (auto const& a : loc.areas) {
        areas.push_back(area_to_json(a));
    }
    j["areas"] = areas;
    
    // Tokens
    json tokens = json::array();
    for (auto const& t : loc.tokens) {
        tokens.push_back(token_to_json(t));
    }
    j["tokens"] = tokens;
    
    // Modes
    if (loc.modes) {
        j["modes"] = *loc.modes;
    }
    
    // Importance
    if (loc.importance) {
        j["importance"] = *loc.importance;
    }
    
    // Address fields
    if (loc.street) {
        j["street"] = *loc.street;
    }
    if (loc.house_number) {
        j["house_number"] = *loc.house_number;
    }
    if (loc.country) {
        j["country"] = *loc.country;
    }
    if (loc.zip) {
        j["zip"] = *loc.zip;
    }
    
    return j;
}

void send_response(const json& data) {
    json resp = {{"status", "ok"}, {"data", data}};
    std::cout << resp.dump() << std::endl;
}

void send_error(std::string const& msg) {
    json resp = {{"status", "error"}, {"message", msg}};
    std::cout << resp.dump() << std::endl;
}

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " <data_path>\n";
        return 1;
    }
    
    std::string data_path = argv[1];
    
    // Initialize MOTIS
    auto* inst = init(data_path);
    if (!inst) {
        send_error("Failed to initialize MOTIS");
        return 1;
    }
    
    // JSON IPC loop
    std::string line;
    while (std::getline(std::cin, line)) {
        try {
            auto req = json::parse(line);
            std::string cmd = req.value("cmd", "");
            
            if (cmd == "geocode") {
                std::string query = req.value("query", "");
                auto locations = geocode(*inst, query);
                
                json result = json::array();
                for (auto const& loc : locations) {
                    result.push_back(location_to_json(loc));
                }
                send_response(result);
            }
            else if (cmd == "plan_route") {
                coord from{req["from_lat"], req["from_lon"]};
                coord to{req["to_lat"], req["to_lon"]};
                
                auto routes = plan_route(*inst, from, to);
                
                json result = json::array();
                for (auto const& r : routes) {
                    result.push_back(route_to_json(r));
                }
                send_response(result);
            }
            else if (cmd == "reverse_geocode") {
                coord pos{req["lat"], req["lon"]};
                auto loc = reverse_geocode(*inst, pos);
                
                if (loc) {
                    send_response(location_to_json(*loc));
                } else {
                    send_response(nullptr);
                }
            }
            else if (cmd == "get_tile") {
                int z = req["z"];
                int x = req["x"];
                int y = req["y"];
                
                auto tile = get_tile(*inst, z, x, y);
                
                if (tile.found) {
                    json result = {
                        {"data_base64", tile.data_base64},
                        {"found", true}
                    };
                    send_response(result);
                } else {
                    json result = {{"found", false}};
                    send_response(result);
                }
            }
            else if (cmd == "get_glyph") {
                std::string path = req.value("path", "");
                if (path.empty()) {
                    send_error("Missing path");
                    continue;
                }

                auto glyph = get_glyph(*inst, path);
                if (glyph.found) {
                    json result = {
                        {"data_base64", glyph.data_base64},
                        {"found", true}
                    };
                    send_response(result);
                } else {
                    json result = {{"found", false}};
                    send_response(result);
                }
            }
            else if (cmd == "api_get") {
                std::string path = req.value("path", "");
                if (path.empty()) {
                    send_error("Missing path");
                    continue;
                }

                auto payload = api_get(*inst, path);
                if (!payload) {
                    send_error("Unsupported endpoint or endpoint failed: " + path);
                    continue;
                }

                auto parsed = json::parse(*payload, nullptr, false);
                if (parsed.is_discarded()) {
                    send_error("Endpoint did not return valid JSON: " + path);
                    continue;
                }
                send_response(parsed);
            }
            else {
                send_error("Unknown command: " + cmd);
            }
        } catch (const std::exception& e) {
            send_error(std::string("Error: ") + e.what());
        }
    }
    
    destroy(inst);
    return 0;
}
