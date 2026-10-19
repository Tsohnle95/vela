export default {
 name:'Fine scanner',family:'Surface',description:'A fine scan line crosses the cube directly under the pearl’s projected position.',rule:'The line moves vertically with the pearl and appears only within range.',
 setup({path,faceEffects,set}){const lines=faceEffects.map(g=>path(g,'M-220 0H220','#f6ecff',3));return ({i,x,y,dy,strength})=>set(lines[i],{transform:`translate(${x} ${y+dy*.65}) rotate(-29)`,opacity:strength*.4});}
};
