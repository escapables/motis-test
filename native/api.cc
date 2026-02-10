#include "native/api.h"

#include <cctype>
#include <iomanip>
#include <sstream>

#include "boost/json.hpp"
#include "boost/url/url_view.hpp"
#include "boost/algorithm/string.hpp"

#include "net/web_server/url_decode.h"

#include "motis/config.h"
#include "motis/data.h"
#include "motis/tiles_data.h"
#include "motis/endpoints/initial.h"
#include "motis/endpoints/levels.h"
#include "motis/endpoints/stop_times.h"
#include "motis/endpoints/trip.h"
#include "motis/endpoints/one_to_all.h"
#include "motis/endpoints/one_to_many.h"
#include "motis/endpoints/routing.h"
#include "motis/endpoints/map/stops.h"
#include "motis/endpoints/map/trips.h"
#include "motis/endpoints/map/rental.h"
#include "motis/endpoints/adr/geocode.h"
#include "motis/endpoints/adr/reverse_geocode.h"
#include "tiles/get_tile.h"
#include "tiles/parse_tile_url.h"
#include "pbf_sdf_fonts_res.h"

// Base64 encoding for tile data
constexpr auto kBase64Chars =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

static std::string base64_encode(std::string const& input) {
  std::string encoded;
  int i = 0;
  unsigned char char_array_3[3];
  unsigned char char_array_4[4];

  for (size_t in_len = input.size(), pos = 0; in_len--; ) {
    char_array_3[i++] = input[pos++];
    if (i == 3) {
      char_array_4[0] = (char_array_3[0] & 0xfc) >> 2;
      char_array_4[1] =
          ((char_array_3[0] & 0x03) << 4) + ((char_array_3[1] & 0xf0) >> 4);
      char_array_4[2] =
          ((char_array_3[1] & 0x0f) << 2) + ((char_array_3[2] & 0xc0) >> 6);
      char_array_4[3] = char_array_3[2] & 0x3f;

      for (auto j = 0; j < 4; ++j) {
        encoded += kBase64Chars[char_array_4[j]];
      }
      i = 0;
    }
  }

  if (i) {
    for (auto j = i; j < 3; ++j) {
      char_array_3[j] = '\0';
    }

    char_array_4[0] = (char_array_3[0] & 0xfc) >> 2;
    char_array_4[1] =
        ((char_array_3[0] & 0x03) << 4) + ((char_array_3[1] & 0xf0) >> 4);
    char_array_4[2] =
        ((char_array_3[1] & 0x0f) << 2) + ((char_array_3[2] & 0xc0) >> 6);
    char_array_4[3] = char_array_3[2] & 0x3f;

    for (auto j = 0; j < (i + 1); ++j) {
      encoded += kBase64Chars[char_array_4[j]];
    }

    while (i < 3) {
      ++i;
      encoded += '=';
    }
  }

  return encoded;
}

namespace motis::native {

struct native_instance {
  explicit native_instance(std::string const& data_path);

  motis::data data_;
  motis::config config_;
};

native_instance::native_instance(std::string const& data_path)
    : data_(data_path,
            config::read(std::filesystem::path{data_path} / "config.yml")),
      config_(data_.config_) {}

native_instance* init(std::string const& data_path) {
  return new native_instance(data_path);
}

void destroy(native_instance* inst) {
  delete inst;
}

// Helper to URL-encode a string
static std::string url_encode(std::string const& value) {
  std::ostringstream escaped;
  escaped.fill('0');
  escaped << std::hex;

  for (char c : value) {
    if (std::isalnum(static_cast<unsigned char>(c)) || c == '-' || c == '_' ||
        c == '.' || c == '~') {
      escaped << c;
    } else {
      escaped << std::uppercase;
      escaped << '%' << std::setw(2)
              << static_cast<int>(static_cast<unsigned char>(c));
      escaped << std::nouppercase;
    }
  }

  return escaped.str();
}

// Helper to build URL query string
static std::string build_route_url(coord from,
                                   coord to,
                                   std::optional<std::string> const& time) {
  std::ostringstream url;
  url << "/api/v1/plan?fromPlace=" << from.lat << "," << from.lon
      << "&toPlace=" << to.lat << "," << to.lon;
  if (time) {
    url << "&time=" << url_encode(*time);
  }
  return url.str();
}

// Helper to convert API mode enum to string
static std::string mode_to_string(api::ModeEnum mode) {
  switch (mode) {
    case api::ModeEnum::WALK: return "WALK";
    case api::ModeEnum::BIKE: return "BIKE";
    case api::ModeEnum::CAR: return "CAR";
    case api::ModeEnum::CAR_PARKING: return "CAR_PARKING";
    case api::ModeEnum::RENTAL: return "RENTAL";
    case api::ModeEnum::TRANSIT: return "TRANSIT";
    case api::ModeEnum::CABLE_CAR: return "CABLE_CAR";
    case api::ModeEnum::FUNICULAR: return "FUNICULAR";
    case api::ModeEnum::RIDE_SHARING: return "RIDE_SHARING";
    default: return "UNKNOWN";
  }
}

std::vector<route> plan_route(native_instance& inst,
                              coord from,
                              coord to,
                              std::optional<std::string> departure_time) {
  std::vector<route> results;

  auto url_str = build_route_url(from, to, departure_time);
  auto url = boost::urls::url_view(url_str);

  // Match the exact order expected by routing constructor
  auto router = motis::ep::routing{
      inst.config_,
      inst.data_.w_.get(),
      inst.data_.l_.get(),
      inst.data_.pl_.get(),
      inst.data_.elevations_.get(),
      inst.data_.tt_.get(),
      inst.data_.tbd_.get(),
      inst.data_.tags_.get(),
      inst.data_.location_rtree_.get(),
      inst.data_.flex_areas_.get(),
      inst.data_.matches_.get(),
      inst.data_.way_matches_.get(),
      inst.data_.rt_,
      inst.data_.shapes_.get(),
      inst.data_.gbfs_,
      inst.data_.adr_ext_.get(),
      inst.data_.tz_.get(),
      inst.data_.odm_bounds_.get(),
      inst.data_.ride_sharing_bounds_.get(),
      inst.data_.metrics_.get()};

  try {
    auto response = router(url);

    for (auto const& itin : response.itineraries_) {
      route r;
      r.duration_seconds = static_cast<int>(itin.duration_);
      r.transfers = static_cast<int>(itin.transfers_);

      for (auto const& leg_obj : itin.legs_) {
        leg l;
        l.mode = mode_to_string(leg_obj.mode_);
        l.from.lat = leg_obj.from_.lat_;
        l.from.lon = leg_obj.from_.lon_;
        l.to.lat = leg_obj.to_.lat_;
        l.to.lon = leg_obj.to_.lon_;
        l.from_name = leg_obj.from_.name_;
        l.to_name = leg_obj.to_.name_;
        l.duration_seconds = static_cast<int>(leg_obj.duration_);
        l.distance_meters =
            leg_obj.distance_ ? static_cast<int>(*leg_obj.distance_) : 0;
        if (leg_obj.routeShortName_) {
          l.route_short_name = *leg_obj.routeShortName_;
        }
        if (leg_obj.headsign_) {
          l.headsign = *leg_obj.headsign_;
        }
        r.legs.push_back(std::move(l));
      }
      results.push_back(std::move(r));
    }
  } catch (std::exception const& e) {
    std::cerr << "Route planning error: " << e.what() << "\n";
  }

  return results;
}

std::vector<location> geocode(native_instance& inst, std::string const& query) {
  std::vector<location> results;

  if (!inst.data_.t_ || !inst.data_.f_ || !inst.data_.tc_) {
    return results;
  }

  std::string url_str = "/api/v1/geocode?text=" + url_encode(query);
  auto url = boost::urls::url_view(url_str);

  auto geocoder = motis::ep::geocode{
      inst.data_.w_.get(),       inst.data_.pl_.get(), inst.data_.matches_.get(),
      inst.data_.tt_.get(),      inst.data_.tags_.get(), *inst.data_.t_,
      *inst.data_.f_,            *inst.data_.tc_, inst.data_.adr_ext_.get()};

  try {
    auto response = geocoder(url);
    for (auto const& place : response) {
      location loc;
      loc.name = place.name_;
      loc.place_id = place.id_;
      loc.pos.lat = place.lat_;
      loc.pos.lon = place.lon_;
      loc.score = place.score_;

      // Type (not optional in Match)
      switch (place.type_) {
        case api::LocationTypeEnum::STOP:
          loc.type = "STOP";
          break;
        case api::LocationTypeEnum::PLACE:
          loc.type = "PLACE";
          break;
        case api::LocationTypeEnum::ADDRESS:
          loc.type = "ADDRESS";
          break;
      }

      // Category
      if (place.category_) {
        loc.category = *place.category_;
      }

      // Areas
      for (auto const& a : place.areas_) {
        area ar;
        ar.name = a.name_;
        ar.admin_level = static_cast<int>(a.adminLevel_);
        ar.matched = a.matched_;
        ar.unique = a.unique_.value_or(false);
        ar.is_default = a.default_.value_or(false);
        loc.areas.push_back(std::move(ar));
      }

      // Tokens
      for (auto const& t : place.tokens_) {
        if (t.size() >= 2) {
          loc.tokens.push_back(
              token{static_cast<int>(t[0]), static_cast<int>(t[1])});
        }
      }

      // Modes
      if (place.modes_) {
        std::vector<std::string> mode_strs;
        for (auto const& m : *place.modes_) {
          mode_strs.push_back(mode_to_string(m));
        }
        loc.modes = std::move(mode_strs);
      }

      // Importance
      if (place.importance_) {
        loc.importance = *place.importance_;
      }

      // Address fields
      if (place.street_) {
        loc.street = *place.street_;
      }
      if (place.houseNumber_) {
        loc.house_number = *place.houseNumber_;
      }
      if (place.country_) {
        loc.country = *place.country_;
      }
      if (place.zip_) {
        loc.zip = *place.zip_;
      }

      results.push_back(std::move(loc));
    }
  } catch (std::exception const& e) {
    std::cerr << "Geocode error: " << e.what() << "\n";
  }

  return results;
}

// Helper to convert API Match to location struct
static location match_to_location(api::Match const& match) {
  location loc;
  loc.name = match.name_;
  loc.place_id = match.id_;
  loc.pos.lat = match.lat_;
  loc.pos.lon = match.lon_;
  loc.score = match.score_;

  // Type (LocationTypeEnum is not optional in Match, it's always present)
  switch (match.type_) {
    case api::LocationTypeEnum::STOP:
      loc.type = "STOP";
      break;
    case api::LocationTypeEnum::PLACE:
      loc.type = "PLACE";
      break;
    case api::LocationTypeEnum::ADDRESS:
      loc.type = "ADDRESS";
      break;
  }

  // Category
  if (match.category_) {
    loc.category = *match.category_;
  }

  // Areas
  for (auto const& a : match.areas_) {
    area ar;
    ar.name = a.name_;
    ar.admin_level = static_cast<int>(a.adminLevel_);
    ar.matched = a.matched_;
    ar.unique = a.unique_.value_or(false);
    ar.is_default = a.default_.value_or(false);
    loc.areas.push_back(std::move(ar));
  }

  // Tokens
  for (auto const& t : match.tokens_) {
    if (t.size() >= 2) {
      loc.tokens.push_back(
          token{static_cast<int>(t[0]), static_cast<int>(t[1])});
    }
  }

  // Modes
  if (match.modes_) {
    std::vector<std::string> mode_strs;
    for (auto const& m : *match.modes_) {
      mode_strs.push_back(mode_to_string(m));
    }
    loc.modes = std::move(mode_strs);
  }

  // Importance
  if (match.importance_) {
    loc.importance = *match.importance_;
  }

  // Address fields
  if (match.street_) {
    loc.street = *match.street_;
  }
  if (match.houseNumber_) {
    loc.house_number = *match.houseNumber_;
  }
  if (match.country_) {
    loc.country = *match.country_;
  }
  if (match.zip_) {
    loc.zip = *match.zip_;
  }

  return loc;
}

std::optional<location> reverse_geocode(native_instance& inst, coord pos) {
  if (!inst.data_.r_ || !inst.data_.t_ || !inst.data_.f_) {
    return std::nullopt;
  }

  std::ostringstream url_str;
  url_str << "/api/v1/reverse-geocode?place=" << pos.lat << "," << pos.lon;
  auto url = boost::urls::url_view(url_str.str());

  auto reverse = motis::ep::reverse_geocode{
      inst.data_.w_.get(),
      inst.data_.pl_.get(),
      inst.data_.matches_.get(),
      inst.data_.tt_.get(),
      inst.data_.tags_.get(),
      *inst.data_.t_,
      *inst.data_.f_,
      *inst.data_.r_,
      inst.data_.adr_ext_.get()};

  try {
    auto response = reverse(url);
    if (response.empty()) {
      return std::nullopt;
    }
    return match_to_location(response[0]);
  } catch (std::exception const& e) {
    std::cerr << "Reverse geocode error: " << e.what() << "\n";
    return std::nullopt;
  }
}

tile_result get_tile(native_instance& inst, int z, int x, int y) {
  tile_result result;
  result.found = false;

  if (!inst.data_.tiles_) {
    std::cerr << "Tiles data not available\n";
    return result;
  }

  try {
    // Create tile coordinates
    geo::tile tile_coord{static_cast<uint32_t>(x), static_cast<uint32_t>(y),
                         static_cast<unsigned>(z)};

    // Use tiles library directly
    auto pc = ::tiles::null_perf_counter{};
    auto rendered_tile = ::tiles::get_tile(
        inst.data_.tiles_->db_handle_,
        inst.data_.tiles_->pack_handle_,
        inst.data_.tiles_->render_ctx_,
        tile_coord,
        pc);

    if (rendered_tile) {
      result.data_base64 = base64_encode(*rendered_tile);
      result.found = true;
    }

  } catch (std::exception const& e) {
    std::cerr << "Tile fetch error: " << e.what() << "\n";
  }

  return result;
}

glyph_result get_glyph(native_instance& /*inst*/, std::string const& path) {
  glyph_result result;
  result.found = false;

  try {
    std::string decoded;
    net::url_decode(path, decoded);

    constexpr auto kPrefix = "/tiles/glyphs/";
    if (!decoded.starts_with(kPrefix)) {
      return result;
    }

    auto res_name = decoded.substr(std::char_traits<char>::length(kPrefix));

    // Keep compatibility with styles that still reference the legacy display font name.
    constexpr auto kDisplay = " Display";
    if (auto const display_pos = res_name.find(kDisplay);
        display_pos != std::string::npos) {
      res_name.erase(display_pos, std::char_traits<char>::length(kDisplay));
    }

    auto const mem = pbf_sdf_fonts_res::get_resource(res_name);
    auto const payload = std::string{
        reinterpret_cast<char const*>(mem.ptr_), static_cast<std::size_t>(mem.size_)};
    result.data_base64 = base64_encode(payload);
    result.found = true;
  } catch (std::out_of_range const&) {
    // Glyph not found in embedded resources.
  } catch (std::exception const& e) {
    std::cerr << "Glyph fetch error: " << e.what() << "\n";
  }

  return result;
}

std::optional<std::string> api_get(native_instance& inst,
                                   std::string const& path_and_query) {
  try {
    auto const url = boost::urls::url_view{path_and_query};
    auto const path = std::string{url.path()};

    if (path == "/api/v1/plan" || path == "/api/v5/plan") {
      if (!inst.data_.w_ || !inst.data_.l_ || !inst.data_.pl_ || !inst.data_.tt_ ||
          !inst.data_.tags_) {
        return std::nullopt;
      }
      auto ep = motis::ep::routing{
          inst.config_,
          inst.data_.w_.get(),
          inst.data_.l_.get(),
          inst.data_.pl_.get(),
          inst.data_.elevations_.get(),
          inst.data_.tt_.get(),
          inst.data_.tbd_.get(),
          inst.data_.tags_.get(),
          inst.data_.location_rtree_.get(),
          inst.data_.flex_areas_.get(),
          inst.data_.matches_.get(),
          inst.data_.way_matches_.get(),
          inst.data_.rt_,
          inst.data_.shapes_.get(),
          inst.data_.gbfs_,
          inst.data_.adr_ext_.get(),
          inst.data_.tz_.get(),
          inst.data_.odm_bounds_.get(),
          inst.data_.ride_sharing_bounds_.get(),
          inst.data_.metrics_.get()};
      return boost::json::serialize(boost::json::value_from(ep(url)));
    }

    if (path == "/api/v1/geocode" || path == "/api/v5/geocode") {
      if (!inst.data_.w_ || !inst.data_.pl_ || !inst.data_.matches_ ||
          !inst.data_.tt_ || !inst.data_.tags_ || !inst.data_.t_ ||
          !inst.data_.f_ || !inst.data_.tc_) {
        return std::nullopt;
      }
      auto ep = motis::ep::geocode{
          inst.data_.w_.get(),      inst.data_.pl_.get(),
          inst.data_.matches_.get(), inst.data_.tt_.get(),
          inst.data_.tags_.get(),    *inst.data_.t_, *inst.data_.f_,
          *inst.data_.tc_,           inst.data_.adr_ext_.get()};
      return boost::json::serialize(boost::json::value_from(ep(url)));
    }

    if (path == "/api/v1/reverse-geocode" || path == "/api/v5/reverse-geocode") {
      if (!inst.data_.w_ || !inst.data_.pl_ || !inst.data_.matches_ ||
          !inst.data_.tt_ || !inst.data_.tags_ || !inst.data_.t_ ||
          !inst.data_.f_ || !inst.data_.r_) {
        return std::nullopt;
      }
      auto ep = motis::ep::reverse_geocode{
          inst.data_.w_.get(),      inst.data_.pl_.get(),
          inst.data_.matches_.get(), inst.data_.tt_.get(),
          inst.data_.tags_.get(),    *inst.data_.t_, *inst.data_.f_,
          *inst.data_.r_,            inst.data_.adr_ext_.get()};
      return boost::json::serialize(boost::json::value_from(ep(url)));
    }

    if (path == "/api/v1/map/initial") {
      if (!inst.data_.tt_) {
        return std::nullopt;
      }
      auto ep = motis::ep::initial{*inst.data_.tt_, inst.config_};
      return boost::json::serialize(boost::json::value_from(ep(url)));
    }

    if (path == "/api/v1/map/levels") {
      if (!inst.data_.w_ || !inst.data_.l_) {
        return std::nullopt;
      }
      auto ep = motis::ep::levels{*inst.data_.w_, *inst.data_.l_};
      return boost::json::serialize(boost::json::value_from(ep(url)));
    }

    if (path == "/api/v1/stoptimes" || path == "/api/v4/stoptimes" ||
        path == "/api/v5/stoptimes") {
      if (!inst.data_.w_ || !inst.data_.pl_ || !inst.data_.matches_ ||
          !inst.data_.tz_ || !inst.data_.location_rtree_ || !inst.data_.tt_ ||
          !inst.data_.tags_) {
        return std::nullopt;
      }
      auto ep = motis::ep::stop_times{
          inst.config_,        inst.data_.w_.get(),   inst.data_.pl_.get(),
          inst.data_.matches_.get(), inst.data_.adr_ext_.get(),
          inst.data_.tz_.get(), *inst.data_.location_rtree_,
          *inst.data_.tt_,     *inst.data_.tags_,      inst.data_.rt_};
      return boost::json::serialize(boost::json::value_from(ep(url)));
    }

    if (path == "/api/v1/trip" || path == "/api/v5/trip") {
      if (!inst.data_.w_ || !inst.data_.l_ || !inst.data_.pl_ ||
          !inst.data_.matches_ || !inst.data_.tt_ || !inst.data_.tags_ ||
          !inst.data_.location_rtree_) {
        return std::nullopt;
      }
      auto ep = motis::ep::trip{
          inst.config_,         inst.data_.w_.get(),    inst.data_.l_.get(),
          inst.data_.pl_.get(), inst.data_.matches_.get(),
          *inst.data_.tt_,      inst.data_.shapes_.get(), inst.data_.adr_ext_.get(),
          inst.data_.tz_.get(), *inst.data_.tags_, *inst.data_.location_rtree_,
          inst.data_.rt_};
      return boost::json::serialize(boost::json::value_from(ep(url)));
    }

    if (path == "/api/v1/map/trips" || path == "/api/v4/map/trips" ||
        path == "/api/v5/map/trips") {
      if (!inst.data_.w_ || !inst.data_.pl_ || !inst.data_.matches_ ||
          !inst.data_.tags_ || !inst.data_.tt_ || !inst.data_.railviz_static_) {
        return std::nullopt;
      }
      auto ep = motis::ep::trips{
          inst.data_.w_.get(),      inst.data_.pl_.get(),
          inst.data_.matches_.get(), inst.data_.adr_ext_.get(), inst.data_.tz_.get(),
          *inst.data_.tags_,        *inst.data_.tt_,            inst.data_.rt_,
          inst.data_.shapes_.get(), *inst.data_.railviz_static_};
      return boost::json::serialize(boost::json::value_from(ep(url)));
    }

    if (path == "/api/v1/map/stops") {
      if (!inst.data_.w_ || !inst.data_.pl_ || !inst.data_.matches_ ||
          !inst.data_.location_rtree_ || !inst.data_.tags_ || !inst.data_.tt_) {
        return std::nullopt;
      }
      auto ep = motis::ep::stops{
          inst.config_,
          inst.data_.w_.get(),
          inst.data_.pl_.get(),
          inst.data_.matches_.get(),
          inst.data_.adr_ext_.get(),
          inst.data_.tz_.get(),
          *inst.data_.location_rtree_,
          *inst.data_.tags_,
          *inst.data_.tt_};
      return boost::json::serialize(boost::json::value_from(ep(url)));
    }

    if (path == "/api/v1/rentals" || path == "/api/v1/map/rentals") {
      auto ep = motis::ep::rental{inst.data_.gbfs_, inst.data_.tt_.get(),
                                  inst.data_.tags_.get()};
      return boost::json::serialize(boost::json::value_from(ep(url)));
    }

    if (path == "/api/v1/one-to-all" || path == "/api/experimental/one-to-all") {
      if (!inst.data_.w_ || !inst.data_.l_ || !inst.data_.pl_ || !inst.data_.tt_ ||
          !inst.data_.tags_) {
        return std::nullopt;
      }
      auto ep = motis::ep::one_to_all{
          inst.config_,
          inst.data_.w_.get(),
          inst.data_.l_.get(),
          inst.data_.pl_.get(),
          inst.data_.elevations_.get(),
          *inst.data_.tt_,
          inst.data_.rt_,
          *inst.data_.tags_,
          inst.data_.flex_areas_.get(),
          inst.data_.location_rtree_.get(),
          inst.data_.matches_.get(),
          inst.data_.adr_ext_.get(),
          inst.data_.tz_.get(),
          inst.data_.way_matches_.get(),
          inst.data_.gbfs_,
          inst.data_.metrics_.get()};
      return boost::json::serialize(boost::json::value_from(ep(url)));
    }

    if (path == "/api/v1/one-to-many") {
      if (!inst.data_.w_ || !inst.data_.l_) {
        return std::nullopt;
      }
      auto ep = motis::ep::one_to_many{*inst.data_.w_, *inst.data_.l_,
                                       inst.data_.elevations_.get()};
      return boost::json::serialize(boost::json::value_from(ep(url)));
    }

  } catch (std::exception const& e) {
    std::cerr << "api_get error: " << e.what() << "\n";
    return std::nullopt;
  }

  return std::nullopt;
}

}  // namespace motis::native
