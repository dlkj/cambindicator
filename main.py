import json
import time

import network
import rp2
import ubinascii
import urequests
import ntptime
import machine
import gc
import micropython

from icalendar import parse

def main():
    rp2.country('GB')

    config_file = open('config.json', 'r')
    config = json.loads(config_file.read())
    config_file.close()

    wlan = network.WLAN(network.STA_IF)
    wlan.active(True)
    print()
    wlan.connect(config.get('ssid'), config.get('password'))

    # Wait for connect or fail
    max_wait = 10
    while max_wait > 0:
        if wlan.status() < 0 or wlan.status() >= 3:
            break
        max_wait -= 1
        print('waiting for connection...')
        time.sleep(1)

    # Handle connection error
    if wlan.status() != 3:
        raise RuntimeError('network connection failed')
    else:
        print('connected')
        status = wlan.ifconfig()
        print('ip = ' + status[0])

    mac = ubinascii.hexlify(network.WLAN().config('mac'), ':').decode()
    print(f"mac: {mac}")

    rtc = machine.RTC()

    ntptime.settime()
    now = rtc.datetime()
    print(now)    

    while True:
        try:
            print(f"mem: {gc.mem_free()} - {gc.mem_alloc()}")
            micropython.mem_info()
            print("Fetching bin calendar...")
            response = urequests.get(
                "https://servicelayer3c.azure-api.net/wastecalendar/calendar/ical/200004185983")
            print("Parsing calendar...")
            (_, events) = parse(response.text.splitlines())
            for e in events:
                print(e)
            print(f"mem: {gc.mem_free()} - {gc.mem_alloc()}")
            micropython.mem_info()
            del e
            response.close()
            del response
            gc.collect()
            print(f"mem: {gc.mem_free()} - {gc.mem_alloc()}")
            micropython.mem_info()
        except ValueError as e:
            print("could not connect (status =" +
                  str(wlan.status()) + ") - " + str(e))
            if wlan.status() < 0 or wlan.status() >= 3:
                print("trying to reconnect...")
                wlan.disconnect()
                wlan.connect(config.get('ssid'), config.get('password'))
                if wlan.status() == 3:
                    print('connected')
                else:
                    print('failed')

        time.sleep(1)


if __name__ == "__main__":
    main()
